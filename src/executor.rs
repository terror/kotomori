use super::*;

type OutputTask = task::JoinHandle<io::Result<String>>;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Executor {
  limits: ExecutionLimit,
}

impl Executor {
  pub(crate) async fn execute(
    &self,
    mut command: tokio::process::Command,
  ) -> ToolResult {
    command.kill_on_drop(true);

    command.stderr(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = match command.spawn() {
      Ok(child) => child,
      Err(error) => return ToolResult::error(error),
    };

    let stdout = child.stdout.take().expect("stdout is piped");
    let stdout = task::spawn(self.read_pipe(stdout));

    let stderr = child.stderr.take().expect("stderr is piped");
    let stderr = task::spawn(self.read_pipe(stderr));

    let status = timeout(self.limits.timeout, child.wait()).await;

    let status = match status {
      Ok(Ok(status)) => status,
      Ok(Err(error)) => return ToolResult::error(error),
      Err(_) => return self.timeout_result(child, stdout, stderr).await,
    };

    let stdout = match stdout.await {
      Ok(Ok(stdout)) => stdout,
      Ok(Err(error)) => return ToolResult::error(error),
      Err(error) => return ToolResult::error(error),
    };

    let stderr = match stderr.await {
      Ok(Ok(stderr)) => stderr,
      Ok(Err(error)) => return ToolResult::error(error),
      Err(error) => return ToolResult::error(error),
    };

    ToolResult::command(status.code(), stdout, stderr)
  }

  async fn read_pipe<R>(self, mut reader: R) -> io::Result<String>
  where
    R: AsyncRead + Send + Unpin + 'static,
  {
    let (mut bytes, mut buffer) = (Vec::new(), [0; 8192]);

    let maximum = self.limits.output_limit.saturating_add(1);

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

    Ok(self.limits.decode(bytes))
  }

  async fn timeout_result(
    &self,
    mut child: tokio::process::Child,
    stdout: OutputTask,
    stderr: OutputTask,
  ) -> ToolResult {
    let kill = child.start_kill();

    let stdout = if let Ok(Ok(Ok(stdout))) =
      timeout(Duration::from_secs(1), stdout).await
    {
      stdout
    } else {
      String::new()
    };

    let stderr = if let Ok(Ok(Ok(stderr))) =
      timeout(Duration::from_secs(1), stderr).await
    {
      stderr
    } else {
      String::new()
    };

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
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn read_pipe_output_is_capped() {
    let executor = Executor {
      limits: ExecutionLimit {
        output_limit: 8,
        timeout: Duration::from_secs(30),
        truncated_marker: "...",
      },
    };

    assert_eq!(
      executor.read_pipe(&b"foo bar baz"[..]).await.unwrap(),
      "foo b..."
    );
  }
}
