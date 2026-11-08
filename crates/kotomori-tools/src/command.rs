use super::*;

#[derive(Clone, Debug)]
pub struct CommandTool {
  pub command_executor: CommandExecutor,
  pub directory: PathBuf,
}

impl Tool for CommandTool {
  fn definition(&self) -> ToolDefinition {
    ToolDefinition {
      description: "Run a command using the system shell and capture stdout, stderr, and exit status. Pipes, redirects, glob expansion, and command chaining are supported.".into(),
      name: "command".into(),
      parameters: serde_json::to_value(CommandArguments::json_schema(
        &mut schemars::SchemaGenerator::default(),
      ))
      .expect("failed to serialize tool schema"),
    }
  }

  fn prepare(&self, arguments: Value) -> Result<ToolCall> {
    let arguments = serde_json::from_value::<CommandArguments>(arguments)?;

    let tool = self.clone();

    Ok(ToolCall {
      approval: ApprovalPolicy::Required,
      future: Box::pin(async move {
        #[cfg(unix)]
        let mut command = {
          let mut command = AsyncCommand::new("/bin/sh");
          command.arg("-c").arg(&arguments.command);
          command
        };

        #[cfg(windows)]
        let mut command = {
          let mut command = AsyncCommand::new("cmd.exe");
          command.arg("/C").arg(&arguments.command);
          command
        };

        command.current_dir(
          tool
            .directory
            .join(arguments.cwd.as_deref().unwrap_or(Path::new("."))),
        );

        tool.command_executor.execute(command).await
      }),
    })
  }
}

#[cfg(test)]
mod tests {
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

  #[tokio::test]
  async fn execute_uses_context_directory() {
    let directory = tempfile::tempdir().unwrap();

    fs::create_dir(directory.path().join("foo")).unwrap();

    fs::write(directory.path().join("bar"), "foo").unwrap();
    fs::write(directory.path().join("foo/bar"), "baz").unwrap();

    let tool = CommandTool {
      command_executor: CommandExecutor::default(),
      directory: directory.path().into(),
    };

    for (cwd, stdout) in [
      (None, "foo"),
      (Some("foo".into()), "baz"),
      (Some(directory.path().join("foo")), "baz"),
    ] {
      assert_eq!(
        tool
          .prepare(json!({
            "command": if cfg!(windows) { "type bar" } else { "cat bar" },
            "cwd": cwd,
          }))
          .unwrap()
          .future
          .await,
        ToolResult {
          exit_status: Some(0),
          outcome: ToolOutcome::Success,
          stdout: Some(stdout.into()),
          ..Default::default()
        },
      );
    }
  }

  #[test]
  fn tool_parameters_are_derived_from_type() {
    let tool = CommandTool {
      command_executor: CommandExecutor::default(),
      directory: "foo".into(),
    }
    .definition();

    assert_eq!(
      tool.parameters,
      json!({
        "type": "object",
        "properties": {
          "command": {"type": "string"},
          "cwd": {"type": ["string", "null"]},
        },
        "required": ["command"],
        "additionalProperties": false,
      }),
    );
  }
}
