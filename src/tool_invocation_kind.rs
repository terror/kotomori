use super::*;

macro_rules! define_tool_invocation_kind {
  ($( $variant:ident($tool:ty), )*) => {
    #[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
    #[serde(untagged)]
    pub(crate) enum ToolInvocationKind {
      $(
        $variant($tool),
      )*
    }

    impl ToolInvocationKind {
      pub(crate) fn action(&self, tense: ToolActionTense) -> &'static str {
        match self {
          $(
            Self::$variant(_) => <$tool>::action(tense),
          )*
        }
      }

      pub(crate) fn arguments(&self) -> Value {
        serde_json::to_value(self)
          .expect("failed to serialize tool arguments")
      }

      pub(crate) fn details(&self) -> Vec<(&'static str, String)> {
        match self {
          $(
            Self::$variant(tool) => tool.details(),
          )*
        }
      }

      pub(crate) fn display(&self) -> String {
        match self {
          $(
            Self::$variant(tool) => tool.display(),
          )*
        }
      }

      pub(crate) async fn execute(
        &self,
        approval: ToolApproval,
      ) -> ToolResult {
        if self.requires_approval() && approval == ToolApproval::Denied {
          return ToolResult::error("permission denied");
        }

        let executor = Executor::default();

        match self {
          $(
            Self::$variant(tool) => tool.execute(&executor).await,
          )*
        }
      }

      pub(crate) fn name(&self) -> &'static str {
        match self {
          $(
            Self::$variant(_) => <$tool>::NAME,
          )*
        }
      }

      pub(crate) fn requires_approval(&self) -> bool {
        match self {
          $(
            Self::$variant(tool) => tool.requires_approval(),
          )*
        }
      }

      pub(crate) fn subject(&self) -> String {
        match self {
          $(
            Self::$variant(tool) => tool.subject(),
          )*
        }
      }
    }
  };
}

define_tools!(define_tool_invocation_kind);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn arguments_serializes_the_wrapped_invocation() {
    let invocation = ToolInvocationKind::Command(CommandTool {
      arguments: vec!["hello".into()],
      cwd: None,
      program: "echo".into(),
    });

    assert_eq!(
      invocation.arguments(),
      serde_json::json!({
        "arguments": ["hello"],
        "program": "echo",
      })
    );
  }

  #[tokio::test]
  async fn denied_approval_short_circuits_approval_required_tools() {
    let invocation = ToolInvocationKind::Command(CommandTool {
      arguments: Vec::new(),
      cwd: None,
      program: "this-command-should-never-run".into(),
    });

    let result = invocation.execute(ToolApproval::Denied).await;

    assert_eq!(result, ToolResult::error("permission denied"));
  }

  #[test]
  fn metadata_methods_delegate_to_the_wrapped_tool() {
    let tool = ReadFileTool {
      cwd: None,
      end_line: Some(10),
      path: "foo.rs".into(),
      start_line: Some(1),
    };

    let invocation = ToolInvocationKind::ReadFile(tool.clone());

    assert_eq!(invocation.name(), ReadFileTool::NAME);
    assert_eq!(invocation.display(), tool.display());
    assert_eq!(invocation.subject(), tool.subject());
    assert_eq!(invocation.details(), tool.details());
  }

  #[test]
  fn mutating_tools_require_approval() {
    let invocation = ToolInvocationKind::ApplyPatch(ApplyPatchTool {
      cwd: None,
      patch: "bar".into(),
    });

    assert!(invocation.requires_approval());
  }

  #[test]
  fn readonly_tools_do_not_require_approval() {
    let invocation = ToolInvocationKind::ReadFile(ReadFileTool {
      cwd: None,
      end_line: None,
      path: "foo".into(),
      start_line: None,
    });

    assert!(!invocation.requires_approval());
  }

  #[test]
  fn search_with_passthrough_flag_requires_approval() {
    let invocation = ToolInvocationKind::SearchFiles(SearchFilesTool {
      arguments: vec!["--pre".into(), "foo".into()],
      cwd: None,
    });

    assert!(invocation.requires_approval());
  }
}
