use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Executor {
  limits: ExecutionLimit,
}

impl Executor {
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
      Err(error) => return ToolResult::error(&error),
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
        let mut stdin =
          child.stdin.take().context("failed to open tool stdin")?;

        stdin.write_all(input.as_bytes()).await?;
      }

      Ok::<_, Error>(child.wait().await?)
    })
    .await;

    let status = match status {
      Ok(Ok(status)) => status,
      Ok(Err(error)) => return ToolResult::error(&error),
      Err(_) => return self.timeout_result(child, stdout, stderr).await,
    };

    let stdout = match stdout {
      Some(stdout) => match stdout.await {
        Ok(Ok(stdout)) => stdout,
        Ok(Err(error)) => return ToolResult::error(&error),
        Err(error) => return ToolResult::error(&error),
      },
      None => String::new(),
    };

    let stderr = match stderr {
      Some(stderr) => match stderr.await {
        Ok(Ok(stderr)) => stderr,
        Ok(Err(error)) => return ToolResult::error(&error),
        Err(error) => return ToolResult::error(&error),
      },
      None => String::new(),
    };

    ToolResult::command(status.code(), stdout, stderr)
  }

  pub(crate) fn new() -> Self {
    Self {
      limits: ExecutionLimit::default(),
    }
  }

  pub(crate) async fn read_file(&self, path: PathBuf) -> ToolResult {
    let limits = self.limits;

    let read = task::spawn_blocking(move || -> io::Result<String> {
      let file = File::open(path)?;

      let mut file = file.take(limits.output_limit as u64 + 1);
      let mut bytes = Vec::new();

      file.read_to_end(&mut bytes)?;

      Ok(limits.decode(bytes))
    });

    match timeout(limits.timeout, async { Ok::<_, Error>(read.await??) }).await
    {
      Ok(Ok(content)) => ToolResult::content(content),
      Ok(Err(error)) => ToolResult::error(&error),
      Err(_) => ToolResult::error(&format!(
        "tool timed out after {} seconds",
        limits.timeout.as_secs()
      )),
    }
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
    let kill = child.kill().await;

    let stdout = match stdout {
      Some(stdout) => match stdout.await {
        Ok(Ok(stdout)) => stdout,
        Ok(Err(_)) | Err(_) => String::new(),
      },
      None => String::new(),
    };

    let stderr = match stderr {
      Some(stderr) => match stderr.await {
        Ok(Ok(stderr)) => stderr,
        Ok(Err(_)) | Err(_) => String::new(),
      },
      None => String::new(),
    };

    if let Err(error) = kill {
      return ToolResult::error(&error);
    }

    let timeout = format!(
      "tool timed out after {} seconds",
      self.limits.timeout.as_secs()
    );

    ToolResult::command(
      None,
      stdout,
      if stderr.is_empty() {
        timeout
      } else {
        format!("{timeout}\n{stderr}")
      },
    )
  }

  #[cfg(test)]
  pub(crate) fn with_limits(limits: ExecutionLimit) -> Self {
    Self { limits }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn read_file_output_is_capped() {
    let executor = Executor::with_limits(ExecutionLimit {
      output_limit: 8,
      timeout: Duration::from_secs(30),
      truncated_marker: "...",
    });

    let path = env::temp_dir().join(format!(
      "kotomori-{}-read-file-output-is-capped",
      process::id()
    ));

    std::fs::write(&path, "foo bar baz").unwrap();

    let result = executor.read_file(path.clone()).await;

    std::fs::remove_file(path).unwrap();

    assert_eq!(result.content.unwrap(), "foo b...");
  }
}
