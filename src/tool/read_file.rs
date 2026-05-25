use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReadFile {
  path: PathBuf,
}

impl Tool for ReadFile {
  const DESCRIPTION: &'static str = "Read a UTF-8 text file.";

  const NAME: &'static str = "read_file";

  fn action(self) -> ToolAction {
    ToolAction::ReadFile { path: self.path }
  }

  fn parameters() -> Value {
    json!({
      "type": "object",
      "properties": {
        "path": {"type": "string"}
      },
      "required": ["path"],
      "additionalProperties": false
    })
  }
}
