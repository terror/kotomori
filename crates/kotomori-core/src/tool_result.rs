use super::*;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolResult {
  pub content: Option<String>,
  pub exit_status: Option<i32>,
  pub outcome: ToolOutcome,
  pub stderr: Option<String>,
  pub stdout: Option<String>,
}

impl ToolResult {
  #[must_use]
  pub fn message_content(&self) -> String {
    serde_json::to_string(self).expect("failed to serialize tool result")
  }

  #[must_use]
  pub fn output(&self) -> Option<String> {
    let output = [
      self.stdout.as_deref(),
      self.content.as_deref(),
      self.stderr.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n");

    (!output.is_empty()).then_some(output)
  }
}
