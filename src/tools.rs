use super::*;

macro_rules! define_tools {
  ($macro:ident) => {
    $macro! {
      Command(CommandTool),
    }
  };
}

macro_rules! impl_from_tools {
  ($( $variant:ident($tool:ty), )*) => {
    $(
      impl From<$tool> for ToolInvocationKind {
        fn from(tool: $tool) -> Self {
          Self::$variant(tool)
        }
      }
    )*
  };
}

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
impl ToolSpec for CommandTool {
  const DESCRIPTION: &'static str = "Run a command using the system shell and capture stdout, stderr, and exit status. Pipes, redirects, glob expansion, and command chaining are supported.";

  const NAME: &'static str = "command";

  fn action(tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Completed => "Ran",
      ToolActionTense::Failed => "Failed running",
      ToolActionTense::Progressive => "Running",
    }
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    self
      .cwd
      .as_ref()
      .map(|cwd| ("cwd", cwd.display().to_string()))
      .into_iter()
      .collect()
  }

  fn display(&self) -> String {
    self.to_string()
  }

  async fn execute(&self, executor: &Executor) -> ToolResult {
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

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    executor.execute(command).await
  }

  fn requires_approval(&self) -> bool {
    true
  }

  fn subject(&self) -> String {
    self.to_string()
  }
}

define_tools!(impl_from_tools);

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn tool_parameters_are_derived_from_type() {
    let tool = Tool::new::<CommandTool>();

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
