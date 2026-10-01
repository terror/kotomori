use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ToolInvocation {
  pub(crate) kind: ToolInvocationKind,
  pub(crate) protocol: ::rig::message::ToolCall,
}

impl ToolInvocation {
  #[cfg(test)]
  pub(crate) fn new(id: &str, kind: ToolInvocationKind) -> Self {
    let protocol = ::rig::message::ToolCall::new(
      id.into(),
      ToolFunction {
        arguments: kind.arguments(),
        name: kind.name().into(),
      },
    );

    Self { kind, protocol }
  }

  pub(crate) fn title(&self, tense: ToolActionTense) -> String {
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
    let protocol = ::rig::message::ToolCall::new(
      "foo".into(),
      ToolFunction {
        arguments: json!({"command": "bar baz", "cwd": null}),
        name: "command".into(),
      },
    )
    .with_call_id("qux".into())
    .with_signature(Some("quux".into()))
    .with_additional_params(Some(json!({"foo": "baz"})));

    let invocation = ToolInvocationKind::decode(protocol.clone()).unwrap();

    assert_eq!(
      invocation,
      ToolInvocation {
        kind: ToolInvocationKind::Command(CommandTool {
          command: "bar baz".into(),
          cwd: None,
        }),
        protocol,
      },
    );
  }

  #[test]
  fn unknown_tool_errors() {
    let error = ToolInvocationKind::decode(::rig::message::ToolCall::new(
      "foo".into(),
      ToolFunction {
        arguments: json!({}),
        name: "bar".into(),
      },
    ))
    .unwrap_err();

    assert_eq!(error.to_string(), "unknown tool `bar`");
  }
}
