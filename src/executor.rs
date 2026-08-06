use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Executor {
  limits: ExecutionLimit,
}

impl Executor {
  async fn collect_output(
    output: Option<task::JoinHandle<io::Result<String>>>,
  ) -> Result<String, Error> {
    match output {
      Some(output) => Ok(output.await??),
      None => Ok(String::new()),
    }
  }

  async fn collect_output_lossy(
    output: Option<task::JoinHandle<io::Result<String>>>,
  ) -> String {
    let Some(output) = output else {
      return String::new();
    };

    match timeout(Duration::from_secs(1), output).await {
      Ok(Ok(Ok(output))) => output,
      Ok(Ok(Err(_)) | Err(_)) | Err(_) => String::new(),
    }
  }

  pub(crate) async fn execute(
    &self,
    mut command: tokio::process::Command,
    input: Option<String>,
  ) -> ToolResult {
    command.kill_on_drop(true);

    command.stderr(Stdio::piped());
    command.stdout(Stdio::piped());

    if input.is_some() {
      command.stdin(Stdio::piped());
    }

    let mut child = match command.spawn() {
      Ok(child) => child,
      Err(error) => return ToolResult::error(error),
    };

    let stdout = child
      .stdout
      .take()
      .map(|stdout| task::spawn(Self::read_pipe(stdout, self.limits)));

    let stderr = child
      .stderr
      .take()
      .map(|stderr| task::spawn(Self::read_pipe(stderr, self.limits)));

    let status = timeout(self.limits.timeout, async {
      if let Some(input) = input {
        let stdin = child.stdin.take().context("failed to open tool stdin")?;
        Self::write_input(stdin, input).await?;
      }

      Ok::<_, Error>(child.wait().await?)
    })
    .await;

    let status = match status {
      Ok(Ok(status)) => status,
      Ok(Err(error)) => return ToolResult::error(error),
      Err(_) => return self.timeout_result(child, stdout, stderr).await,
    };

    let stdout = match Self::collect_output(stdout).await {
      Ok(stdout) => stdout,
      Err(error) => return ToolResult::error(error),
    };

    let stderr = match Self::collect_output(stderr).await {
      Ok(stderr) => stderr,
      Err(error) => return ToolResult::error(error),
    };

    ToolResult::command(status.code(), stdout, stderr)
  }

  async fn read_pipe<R>(
    mut reader: R,
    limits: ExecutionLimit,
  ) -> io::Result<String>
  where
    R: AsyncRead + Send + Unpin + 'static,
  {
    let (mut bytes, mut buffer) = (Vec::new(), [0; 8192]);

    let maximum = limits.output_limit.saturating_add(1);

    loop {
      let count = reader.read(&mut buffer).await?;

      if count == 0 {
        break;
      }

      let remaining = maximum.saturating_sub(bytes.len());

      if remaining == 0 {
        continue;
      }

      if count > remaining {
        bytes.extend_from_slice(&buffer[..remaining]);
      } else {
        bytes.extend_from_slice(&buffer[..count]);
      }
    }

    Ok(limits.decode(bytes))
  }

  async fn timeout_result(
    &self,
    mut child: tokio::process::Child,
    stdout: Option<task::JoinHandle<io::Result<String>>>,
    stderr: Option<task::JoinHandle<io::Result<String>>>,
  ) -> ToolResult {
    let kill = child.start_kill();

    let stdout = Self::collect_output_lossy(stdout).await;
    let stderr = Self::collect_output_lossy(stderr).await;

    let wait = timeout(Duration::from_secs(1), child.wait()).await;

    let timeout = format!(
      "tool timed out after {} seconds",
      self.limits.timeout.as_secs()
    );

    let mut stderr = if stderr.is_empty() {
      timeout
    } else {
      format!("{timeout}\n{stderr}")
    };

    if let Err(error) = kill {
      stderr.push_str("\nfailed to kill process: ");
      stderr.push_str(&error.to_string());
    }

    match wait {
      Ok(Ok(_)) => {}
      Ok(Err(error)) => {
        stderr.push_str("\nfailed to wait for process: ");
        stderr.push_str(&error.to_string());
      }
      Err(_) => {
        stderr.push_str("\nfailed to wait for process: timed out");
      }
    }

    ToolResult::command(None, stdout, stderr)
  }

  async fn write_input<W>(
    mut stdin: W,
    input: impl AsRef<[u8]>,
  ) -> Result<(), Error>
  where
    W: AsyncWrite + Unpin,
  {
    stdin.write_all(input.as_ref()).await?;
    stdin.shutdown().await?;

    drop(stdin);

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn collect_output_lossy_returns_output() {
    let output = task::spawn(async { Ok::<_, io::Error>("foo".to_string()) });

    let output = Executor::collect_output_lossy(Some(output)).await;

    assert_eq!(output, "foo");
  }

  #[tokio::test]
  async fn read_pipe_output_is_capped() {
    let limits = ExecutionLimit {
      output_limit: 8,
      timeout: Duration::from_secs(30),
      truncated_marker: "...",
    };

    let output = Executor::read_pipe(&b"foo bar baz"[..], limits)
      .await
      .unwrap();

    assert_eq!(output, "foo b...");
  }

  #[tokio::test]
  async fn write_input_closes_stdin() {
    let (stdin, mut stdout) = tokio::io::duplex(1024);

    Executor::write_input(stdin, "foo").await.unwrap();

    let mut output = String::new();

    stdout.read_to_string(&mut output).await.unwrap();

    assert_eq!(output, "foo");
  }
}
