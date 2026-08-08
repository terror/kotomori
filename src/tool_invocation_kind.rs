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

      pub(crate) async fn execute(&self, executor: &Executor) -> ToolResult {
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
      command: "echo hello".into(),
      cwd: None,
    });

    assert_eq!(
      invocation.arguments(),
      serde_json::json!({
        "command": "echo hello",
      })
    );
  }

  #[test]
  fn command_requires_approval() {
    let invocation = ToolInvocationKind::Command(CommandTool {
      command: "foo".into(),
      cwd: None,
    });

    assert!(invocation.requires_approval());
  }

  #[test]
  fn metadata_methods_delegate_to_the_wrapped_tool() {
    let tool = CommandTool {
      command: "foo bar".into(),
      cwd: None,
    };

    let invocation = ToolInvocationKind::Command(tool.clone());

    assert_eq!(invocation.name(), CommandTool::NAME);
    assert_eq!(invocation.display(), tool.display());
    assert_eq!(invocation.subject(), tool.subject());
    assert_eq!(invocation.details(), tool.details());
  }
}
