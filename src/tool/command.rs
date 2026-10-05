use super::*;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandTool {
  pub(crate) command: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
}

impl Display for CommandTool {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", self.command)
  }
}

#[async_trait]
impl ToolCall for CommandTool {
  const DESCRIPTION: &'static str = "Run a command using the system shell and capture stdout, stderr, and exit status. Pipes, redirects, glob expansion, and command chaining are supported.";

  const NAME: &'static str = "command";

  fn action(tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Completed => "Ran",
      ToolActionTense::Failed => "Failed running",
      ToolActionTense::Progressive => "Running",
    }
  }

  fn approval(&self) -> ApprovalPolicy {
    ApprovalPolicy::Required
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    self
      .cwd
      .as_ref()
      .map(|cwd| ("cwd", cwd.display().to_string()))
      .into_iter()
      .collect()
  }

  async fn execute(&self, context: &ToolContext) -> ToolResult {
    #[cfg(unix)]
    let mut command = {
      let mut command = AsyncCommand::new("/bin/sh");
      command.arg("-c").arg(&self.command);
      command
    };

    #[cfg(windows)]
    let mut command = {
      let mut command = AsyncCommand::new("cmd.exe");
      command.arg("/C").arg(&self.command);
      command
    };

    command.current_dir(
      context
        .directory
        .join(self.cwd.as_deref().unwrap_or(Path::new("."))),
    );

    context.command_executor.execute(command).await
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[tokio::test]
  async fn execute_uses_context_directory() {
    let directory = tempfile::tempdir().unwrap();

    fs::create_dir(directory.path().join("foo")).unwrap();
    fs::write(directory.path().join("bar"), "foo").unwrap();
    fs::write(directory.path().join("foo/bar"), "baz").unwrap();

    let context = ToolContext {
      command_executor: CommandExecutor::default(),
      directory: directory.path().into(),
    };

    for (cwd, stdout) in [
      (None, "foo"),
      (Some("foo".into()), "baz"),
      (Some(directory.path().join("foo")), "baz"),
    ] {
      assert_eq!(
        CommandTool {
          command: if cfg!(windows) { "type bar" } else { "cat bar" }.into(),
          cwd,
        }
        .execute(&context)
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
    let tool = ToolInvocationKind::definitions().remove(0);

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
