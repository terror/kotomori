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

  pub(crate) async fn execute(&self, approval: ToolApproval) -> ToolResult {
    if self.requires_approval() && approval == ToolApproval::Denied {
      return ToolResult::error("permission denied");
    }

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

  pub(crate) fn requires_approval(&self) -> bool {
    match self {
      Self::ApplyPatch(_) | Self::Command(_) | Self::WriteFile(_) => true,
      Self::SearchFiles(tool) => tool
        .arguments
        .iter()
        .any(|argument| argument == "--pre" || argument.starts_with("--pre=")),
      _ => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn approval_requirements() {
    assert!(
      ToolInvocationKind::ApplyPatch(ApplyPatchTool {
        cwd: None,
        patch: "foo".into(),
      })
      .requires_approval()
    );

    assert!(
      ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "foo".into(),
      })
      .requires_approval()
    );

    assert!(
      !ToolInvocationKind::ListFiles(ListFilesTool { cwd: None })
        .requires_approval()
    );

    assert!(
      !ToolInvocationKind::ReadFile(ReadFileTool {
        cwd: None,
        end_line: None,
        path: "foo".into(),
        start_line: None,
      })
      .requires_approval()
    );

    assert!(
      !ToolInvocationKind::SearchFiles(SearchFilesTool {
        arguments: Vec::new(),
        cwd: None,
      })
      .requires_approval()
    );

    assert!(
      ToolInvocationKind::SearchFiles(SearchFilesTool {
        arguments: vec!["--pre".into(), "foo".into()],
        cwd: None,
      })
      .requires_approval()
    );

    assert!(
      ToolInvocationKind::WriteFile(WriteFileTool {
        content: "bar".into(),
        cwd: None,
        path: "foo".into(),
      })
      .requires_approval()
    );
  }

  #[tokio::test]
  async fn denied_command_does_not_execute() {
    let result = ToolInvocationKind::Command(CommandTool {
      arguments: Vec::new(),
      cwd: None,
      program: "bar".into(),
    })
    .execute(ToolApproval::Denied)
    .await;

    assert_eq!(result, ToolResult::error("permission denied"));
  }
}
