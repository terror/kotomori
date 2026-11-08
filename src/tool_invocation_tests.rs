use {super::*, serde_json::json};

#[test]
fn decodes_command_tool_call() {
  let protocol = ToolRequest {
    additional_params: Some(json!({"foo": "baz"})),
    function: ToolFunction {
      arguments: json!({"command": "bar baz", "cwd": null}),
      name: ToolName::new("command").unwrap(),
    },
    id: CallId::Provider {
      call_id: "qux".into(),
      item_id: Some("foo".into()),
    },
    signature: Some("quux".into()),
  };

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
