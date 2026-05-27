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
  pub(crate) fn action(&self, tense: ToolActionTense) -> &'static str {
    match self {
      Self::ApplyPatch(_) => ApplyPatchTool::action(tense),
      Self::Command(_) => CommandTool::action(tense),
      Self::ListFiles(_) => ListFilesTool::action(tense),
      Self::ReadFile(_) => ReadFileTool::action(tense),
      Self::SearchFiles(_) => SearchFilesTool::action(tense),
      Self::WriteFile(_) => WriteFileTool::action(tense),
    }
  }

  pub(crate) fn arguments(&self) -> Value {
    serde_json::to_value(self).expect("failed to serialize tool arguments")
  }

  pub(crate) fn details(&self) -> Vec<(&'static str, String)> {
    match self {
      Self::ApplyPatch(tool) => tool.details(),
      Self::Command(tool) => tool.details(),
      Self::ListFiles(tool) => tool.details(),
      Self::ReadFile(tool) => tool.details(),
      Self::SearchFiles(tool) => tool.details(),
      Self::WriteFile(tool) => tool.details(),
    }
  }

  pub(crate) fn display(&self) -> String {
    match self {
      Self::ApplyPatch(tool) => tool.display(),
      Self::Command(tool) => tool.display(),
      Self::ListFiles(tool) => tool.display(),
      Self::ReadFile(tool) => tool.display(),
      Self::SearchFiles(tool) => tool.display(),
      Self::WriteFile(tool) => tool.display(),
    }
  }

  pub(crate) async fn execute(&self, approval: ToolApproval) -> ToolResult {
    if self.requires_approval() && approval == ToolApproval::Denied {
      return ToolResult::error("permission denied");
    }

    let executor = Executor::default();

    match self {
      Self::ApplyPatch(tool) => tool.execute(&executor).await,
      Self::Command(tool) => tool.execute(&executor).await,
      Self::ListFiles(tool) => tool.execute(&executor).await,
      Self::ReadFile(tool) => tool.execute(&executor).await,
      Self::SearchFiles(tool) => tool.execute(&executor).await,
      Self::WriteFile(tool) => tool.execute(&executor).await,
    }
  }

  pub(crate) fn name(&self) -> &'static str {
    match self {
      Self::ApplyPatch(_) => ApplyPatchTool::NAME,
      Self::Command(_) => CommandTool::NAME,
      Self::ListFiles(_) => ListFilesTool::NAME,
      Self::ReadFile(_) => ReadFileTool::NAME,
      Self::SearchFiles(_) => SearchFilesTool::NAME,
      Self::WriteFile(_) => WriteFileTool::NAME,
    }
  }

  pub(crate) fn requires_approval(&self) -> bool {
    match self {
      Self::ApplyPatch(tool) => tool.requires_approval(),
      Self::Command(tool) => tool.requires_approval(),
      Self::ListFiles(tool) => tool.requires_approval(),
      Self::ReadFile(tool) => tool.requires_approval(),
      Self::SearchFiles(tool) => tool.requires_approval(),
      Self::WriteFile(tool) => tool.requires_approval(),
    }
  }

  pub(crate) fn subject(&self) -> String {
    match self {
      Self::ApplyPatch(tool) => tool.subject(),
      Self::Command(tool) => tool.subject(),
      Self::ListFiles(tool) => tool.subject(),
      Self::ReadFile(tool) => tool.subject(),
      Self::SearchFiles(tool) => tool.subject(),
      Self::WriteFile(tool) => tool.subject(),
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
