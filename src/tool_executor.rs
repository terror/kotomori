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
