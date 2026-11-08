use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ToolInvocation {
  pub(crate) protocol: ToolRequest,
}

impl ToolInvocation {
  #[cfg(test)]
  pub(crate) fn new(id: &str, arguments: CommandArguments) -> Self {
    let protocol = ToolRequest::new(
      CallId::new(id),
      ToolFunction {
        arguments: serde_json::to_value(arguments).unwrap(),
        name: ToolName::new("command").unwrap(),
      },
    );

    Self { protocol }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn decodes_command_tool_call() {
    let protocol = ToolRequest::new(
      CallId::Provider {
        call_id: "qux".into(),
        item_id: Some("foo".into()),
      },
      ToolFunction {
        arguments: json!({"command": "bar baz", "cwd": null}),
        name: ToolName::new("command").unwrap(),
      },
    )
    .with_signature(Some("quux".into()))
    .with_additional_params(Some(json!({"foo": "baz"})));

    let mut tools = ToolRegistry::default();

    tools
      .register(Arc::new(CommandTool {
        command_executor: CommandExecutor::default(),
        directory: "foo".into(),
      }))
      .unwrap();

    let (invocation, _tool) = tools.prepare(protocol.clone()).unwrap();

    assert_eq!(invocation, ToolInvocation { protocol });
  }
}
