use super::*;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct ToolInvocation {
  pub(crate) id: String,
  pub(crate) kind: ToolInvocationKind,
}

impl ToolInvocation {
  pub(crate) fn completed_tense(&self) -> String {
    self.title(ToolActionTense::Completed)
  }

  pub(crate) fn failed_tense(&self) -> String {
    self.title(ToolActionTense::Failed)
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn title(&self, tense: ToolActionTense) -> String {
    format!("{} {}", self.kind.action(tense), self.kind)
  }
}

impl Display for ToolInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    Display::fmt(&self.kind, f)
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn decodes_command_tool_call() {
    let invocation = ToolInvocationKind::decode(RawToolCall {
      arguments: json!({"command": "bar baz", "cwd": null}),
      id: "foo".into(),
      name: "command".into(),
    })
    .unwrap();

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::Command(CommandTool {
          command: "bar baz".into(),
          cwd: None,
        }),
      },
    );
  }

  #[test]
  fn tagged_kind_round_trips() {
    let kind = ToolInvocationKind::Command(CommandTool {
      command: "echo hello".into(),
      cwd: None,
    });

    let value = json!({
      "name": "command",
      "arguments": {"command": "echo hello"},
    });

    assert_eq!(serde_json::to_value(&kind).unwrap(), value);

    assert_eq!(
      serde_json::from_value::<ToolInvocationKind>(value).unwrap(),
      kind
    );
  }

  #[test]
  fn unknown_tool_errors() {
    let error = ToolInvocationKind::decode(RawToolCall {
      arguments: json!({}),
      id: "foo".into(),
      name: "bar".into(),
    })
    .unwrap_err();

    assert_eq!(error.to_string(), "unknown tool `bar`");
  }
}
