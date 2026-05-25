use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ApplyPatch {
  cwd: Option<PathBuf>,
  patch: String,
}

impl Tool for ApplyPatch {
  const DESCRIPTION: &'static str = "Apply a unified patch to the workspace.";

  const NAME: &'static str = "apply_patch";

  fn action(self) -> ToolAction {
    ToolAction::ApplyPatch {
      cwd: self.cwd,
      patch: self.patch,
    }
  }

  fn parameters() -> Value {
    json!({
      "type": "object",
      "properties": {
        "patch": {"type": "string"},
        "cwd": {"type": ["string", "null"]}
      },
      "required": ["patch"],
      "additionalProperties": false
    })
  }
}
