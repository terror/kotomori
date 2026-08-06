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
  pub(crate) arguments: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) program: String,
}

impl Display for CommandTool {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    if self.arguments.is_empty() {
      write!(f, "{}", self.program)
    } else {
      write!(f, "{} {}", self.program, self.arguments.join(" "))
    }
  }
}

#[async_trait]
impl ToolSpec for CommandTool {
  const DESCRIPTION: &'static str =
    "Run a command and capture stdout, stderr, and exit status.";

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
    let mut command = AsyncCommand::new(&self.program);

    command.args(&self.arguments);

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
          "arguments": {
            "type": "array",
            "items": {"type": "string"},
          },
          "cwd": {"type": ["string", "null"]},
          "program": {"type": "string"},
        },
        "required": ["arguments", "program"],
        "additionalProperties": false,
      }),
    );
  }
}
