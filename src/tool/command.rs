use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
  arguments: Vec<String>,
  cwd: Option<PathBuf>,
  program: String,
}

impl Tool for Command {
  const DESCRIPTION: &'static str = "Run a command and capture stdout, stderr, and exit status. Do not use this to list project files; use list_files instead.";

  const NAME: &'static str = "command";

  fn action(self) -> ToolAction {
    ToolAction::Command(CommandInvocation {
      arguments: self.arguments,
      cwd: self.cwd,
      program: self.program,
    })
  }

  fn parameters() -> Value {
    json!({
      "type": "object",
      "properties": {
        "program": {"type": "string"},
        "arguments": {
          "type": "array",
          "items": {"type": "string"}
        },
        "cwd": {"type": ["string", "null"]}
      },
      "required": ["program", "arguments"],
      "additionalProperties": false
    })
  }
}
