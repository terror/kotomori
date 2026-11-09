use super::*;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommandOutput {
  pub exit_status: Option<i32>,
  pub stderr: String,
  pub stdout: String,
}

impl CommandOutput {
  pub(crate) fn into_result(self, outcome: ToolOutcome) -> ToolResult {
    ToolResult {
      content: [&self.stdout, &self.stderr]
        .into_iter()
        .filter(|output| !output.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n"),
      details: serde_json::to_value(self)
        .expect("failed to serialize command output"),
      outcome,
    }
  }
}
