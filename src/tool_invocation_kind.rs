use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch(ApplyPatchTool),
  Command(CommandTool),
  ListFiles(ListFilesTool),
  ReadFile(ReadFileTool),
  SearchFiles(SearchFilesTool),
  WriteFile(WriteFileTool),
}

impl ToolInvocationKind {
  pub(crate) fn arguments(&self) -> Value {
    serde_json::to_value(self).expect("failed to serialize tool arguments")
  }

  pub(crate) async fn execute(&self) -> ToolResult {
    let executor = Executor::default();

    match self {
      Self::ApplyPatch(tool) => {
        let mut command = tokio::process::Command::new("apply_patch");

        if let Some(cwd) = &tool.cwd {
          command.current_dir(cwd);
        }

        executor.execute(command, Some(tool.patch.clone())).await
      }
      Self::Command(tool) => {
        let mut command = tokio::process::Command::new(&tool.program);

        command.args(&tool.arguments);

        if let Some(cwd) = &tool.cwd {
          command.current_dir(cwd);
        }

        executor.execute(command, None).await
      }
      Self::ListFiles(tool) => {
        let mut command = tokio::process::Command::new("rg");

        command.arg("--files");

        if let Some(cwd) = &tool.cwd {
          command.current_dir(cwd);
        }

        executor.execute(command, None).await
      }
      Self::ReadFile(tool) => executor.read_file(tool).await,
      Self::SearchFiles(tool) => {
        let mut command = tokio::process::Command::new("rg");

        command.args(&tool.arguments);

        if let Some(cwd) = &tool.cwd {
          command.current_dir(cwd);
        }

        executor.execute(command, None).await
      }
      Self::WriteFile(tool) => executor.write_file(tool).await,
    }
  }
}
