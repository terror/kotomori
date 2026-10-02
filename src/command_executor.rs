use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CommandExecutor {
  limits: ExecutionLimit,
}

impl CommandExecutor {
  pub(crate) async fn execute(&self, command: AsyncCommand) -> ToolResult {
    self
      .execute_inner(command)
      .await
      .unwrap_or_else(|error| ToolResult {
        stderr: Some(error.to_string()),
        ..Default::default()
      })
  }

  async fn execute_inner(
    &self,
    mut command: AsyncCommand,
  ) -> Result<ToolResult> {
    command.stderr(Stdio::piped());
    command.stdout(Stdio::piped());

    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());

    let status = timeout(self.limits.timeout, async {
      let mut child = CommandChild::spawn(command)?;

      let (stdout_pipe, stderr_pipe) = (
        child.inner.stdout().take().expect("stdout is piped"),
        child.inner.stderr().take().expect("stderr is piped"),
      );

      tokio::try_join!(
        child.inner.wait(),
        self.read_pipe(stdout_pipe, &mut stdout),
        self.read_pipe(stderr_pipe, &mut stderr),
      )
    })
    .await;

    let (stdout, stderr) = (self.pipe_output(stdout), self.pipe_output(stderr));

    let (exit_status, outcome, stderr) = if let Ok(status) = status {
      let (status, (), ()) = status?;

      (
        status.code(),
        if status.success() {
          ToolOutcome::Success
        } else {
          ToolOutcome::Failure
        },
        stderr,
      )
    } else {
      let timeout = format!(
        "tool timed out after {} seconds",
        self.limits.timeout.as_secs()
      );

      (
        None,
        ToolOutcome::Failure,
        if stderr.is_empty() {
          timeout
        } else {
          format!("{timeout}\n{stderr}")
        },
      )
    };

    Ok(ToolResult {
      exit_status,
      outcome,
      stderr: (!stderr.is_empty()).then_some(stderr),
      stdout: (!stdout.is_empty()).then_some(stdout),
      ..Default::default()
    })
  }

  fn pipe_output(self, mut bytes: Vec<u8>) -> String {
    let truncated = bytes.len() > self.limits.output_limit;

    if truncated {
      bytes.truncate(
        self
          .limits
          .output_limit
          .saturating_sub(self.limits.truncated_marker.len()),
      );
    }

    let mut output = String::from_utf8_lossy(&bytes).into_owned();

    if truncated {
      output.push_str(self.limits.truncated_marker);
    }

    output
  }

  async fn read_pipe<R>(
    self,
    mut reader: R,
    bytes: &mut Vec<u8>,
  ) -> io::Result<()>
  where
    R: AsyncRead + Unpin,
  {
    let mut buffer = vec![0; 8192];

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

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn execute_exit_status() {
    let (shell, flag) = if cfg!(windows) {
      ("cmd.exe", "/C")
    } else {
      ("/bin/sh", "-c")
    };

    for (exit_status, outcome) in
      [(0, ToolOutcome::Success), (1, ToolOutcome::Failure)]
    {
      let mut command = AsyncCommand::new(shell);
      command.arg(flag).arg(format!("exit {exit_status}"));

      assert_eq!(
        CommandExecutor::default().execute(command).await,
        ToolResult {
          exit_status: Some(exit_status),
          outcome,
          ..Default::default()
        },
      );
    }
  }

  #[tokio::test]
  async fn execute_spawn_error() {
    let directory = tempfile::tempdir().unwrap();

    assert_eq!(
      CommandExecutor::default()
        .execute(AsyncCommand::new(directory.path().join("foo")))
        .await,
      ToolResult {
        stderr: Some(io::Error::from_raw_os_error(2).to_string()),
        ..Default::default()
      },
    );
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn execute_timeout() {
    let executor = CommandExecutor {
      limits: ExecutionLimit {
        timeout: Duration::ZERO,
        ..Default::default()
      },
    };

    let mut command = AsyncCommand::new("/bin/sleep");
    command.arg("10");

    assert_eq!(
      executor.execute(command).await,
      ToolResult {
        stderr: Some("tool timed out after 0 seconds".into()),
        ..Default::default()
      },
    );
  }

  #[tokio::test]
  async fn read_pipe_output_is_capped() {
    let executor = CommandExecutor {
      limits: ExecutionLimit {
        output_limit: 8,
        timeout: Duration::from_secs(30),
        truncated_marker: "...",
      },
    };

    let mut bytes = Vec::new();

    executor
      .read_pipe(&b"foo bar baz"[..], &mut bytes)
      .await
      .unwrap();

    assert_eq!(bytes, b"foo bar b");
    assert_eq!(executor.pipe_output(bytes), "foo b...");
  }
}
