use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ToolInvocation {
  pub(crate) kind: ToolInvocationKind,
  pub(crate) protocol: ::rig::message::ToolCall,
}

impl ToolInvocation {
  #[cfg(test)]
  pub(crate) fn new(id: &str, kind: ToolInvocationKind) -> Self {
    let protocol = ::rig::message::ToolCall::from_wire(
      id,
      ToolFunction {
        arguments: kind.arguments(),
        name: ToolName::new(kind.name()).unwrap(),
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
    let protocol = ::rig::message::ToolCall::from_dual_wire(
      "foo",
      "qux",
      ToolFunction {
        arguments: json!({"command": "bar baz", "cwd": null}),
        name: ToolName::new("command").unwrap(),
      },
    )
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
}
