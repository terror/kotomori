use super::*;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ToolResult {
  pub(crate) content: Option<String>,
  pub(crate) exit_status: Option<i32>,
  pub(crate) stderr: Option<String>,
  pub(crate) stdout: Option<String>,
}

impl ToolResult {
  pub(crate) fn is_error(&self) -> bool {
    self.exit_status.is_none_or(|status| status != 0)
  }

  pub(crate) fn message(&self, id: impl Into<String>) -> Message {
    Message::User(vec![UserMessageContent::ToolResult {
      id: id.into(),
      result: self.clone(),
    }])
  }

  pub(crate) fn message_content(&self) -> String {
    serde_json::to_string(self).expect("failed to serialize tool result")
  }

  pub(crate) fn output(&self) -> Option<String> {
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
