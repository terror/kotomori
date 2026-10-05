use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ToolExecutor {
  pub(crate) command_executor: CommandExecutor,
  pub(crate) directory: PathBuf,
}

impl ToolExecutor {
  pub(crate) async fn execute(&self, tool: &ToolInvocationKind) -> ToolResult {
    match tool {
      ToolInvocationKind::Command(tool) => {
        #[cfg(unix)]
        let mut command = {
          let mut command = AsyncCommand::new("/bin/sh");
          command.arg("-c").arg(&tool.command);
          command
        };

        #[cfg(windows)]
        let mut command = {
          let mut command = AsyncCommand::new("cmd.exe");
          command.arg("/C").arg(&tool.command);
          command
        };

        command.current_dir(
          self
            .directory
            .join(tool.cwd.as_deref().unwrap_or(Path::new("."))),
        );

        self.command_executor.execute(command).await
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn execute_uses_context_directory() {
    let directory = tempfile::tempdir().unwrap();

    fs::create_dir(directory.path().join("foo")).unwrap();

    fs::write(directory.path().join("bar"), "foo").unwrap();
    fs::write(directory.path().join("foo/bar"), "baz").unwrap();

    let executor = ToolExecutor {
      command_executor: CommandExecutor::default(),
      directory: directory.path().into(),
    };

    for (cwd, stdout) in [
      (None, "foo"),
      (Some("foo".into()), "baz"),
      (Some(directory.path().join("foo")), "baz"),
    ] {
      assert_eq!(
        executor
          .execute(&ToolInvocationKind::Command(CommandTool {
            command: if cfg!(windows) { "type bar" } else { "cat bar" }.into(),
            cwd,
          }))
          .await,
        ToolResult {
          exit_status: Some(0),
          outcome: ToolOutcome::Success,
          stdout: Some(stdout.into()),
          ..Default::default()
        },
      );
    }
  }
}
