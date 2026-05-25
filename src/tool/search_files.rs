use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SearchFiles {
  arguments: Vec<String>,
  cwd: Option<PathBuf>,
}

impl Tool for SearchFiles {
  const DESCRIPTION: &'static str = "Search files with ripgrep.";

  const NAME: &'static str = "search_files";

  fn action(self) -> ToolAction {
    ToolAction::Command(CommandInvocation {
      arguments: self.arguments,
      cwd: self.cwd,
      program: "rg".into(),
    })
  }

  fn parameters() -> Value {
    json!({
      "type": "object",
      "properties": {
        "arguments": {
          "type": "array",
          "items": {"type": "string"}
        },
        "cwd": {"type": ["string", "null"]}
      },
      "required": ["arguments"],
      "additionalProperties": false
    })
  }
}
