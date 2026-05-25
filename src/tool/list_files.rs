use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ListFiles {
  cwd: Option<PathBuf>,
}

impl Tool for ListFiles {
  const DESCRIPTION: &'static str = "List project files while respecting .gitignore and other standard ignore rules.";

  const NAME: &'static str = "list_files";

  fn action(self) -> ToolAction {
    ToolAction::Command(CommandInvocation {
      arguments: vec!["--files".into()],
      cwd: self.cwd,
      program: "rg".into(),
    })
  }

  fn parameters() -> Value {
    json!({
      "type": "object",
      "properties": {
        "cwd": {"type": ["string", "null"]}
      },
      "required": [],
      "additionalProperties": false
    })
  }
}
