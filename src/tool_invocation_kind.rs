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
        executor: &Executor,
        approval: ToolApproval,
      ) -> ToolResult {
        if self.requires_approval() && approval == ToolApproval::Denied {
          return ToolResult::error("permission denied");
        }

        match self {
          $(
            Self::$variant(tool) => tool.execute(executor).await,
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

  #[test]
  fn command_requires_approval() {
    let invocation = ToolInvocationKind::Command(CommandTool {
      arguments: Vec::new(),
      cwd: None,
      program: "foo".into(),
    });

    assert!(invocation.requires_approval());
  }

  #[tokio::test]
  async fn denied_approval_short_circuits_approval_required_tools() {
    let invocation = ToolInvocationKind::Command(CommandTool {
      arguments: Vec::new(),
      cwd: None,
      program: "this-command-should-never-run".into(),
    });

    let result = invocation
      .execute(&Executor::default(), ToolApproval::Denied)
      .await;

    assert_eq!(result, ToolResult::error("permission denied"));
  }

  #[test]
  fn metadata_methods_delegate_to_the_wrapped_tool() {
    let tool = CommandTool {
      arguments: vec!["bar".into()],
      cwd: None,
      program: "foo".into(),
    };

    let invocation = ToolInvocationKind::Command(tool.clone());

    assert_eq!(invocation.name(), CommandTool::NAME);
    assert_eq!(invocation.display(), tool.display());
    assert_eq!(invocation.subject(), tool.subject());
    assert_eq!(invocation.details(), tool.details());
  }
}
