use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ToolResult {
  content: Option<String>,
  error: Option<String>,
  exit_status: Option<i32>,
  stdout: Option<String>,
}

impl ToolResult {
  pub(crate) fn command(
    exit_status: Option<i32>,
    stdout: impl Into<String>,
    stderr: impl Into<String>,
  ) -> Self {
    let (stdout, stderr) = (stdout.into(), stderr.into());

    Self {
      content: None,
      error: (!stderr.is_empty()).then_some(stderr),
      exit_status,
      stdout: (!stdout.is_empty()).then_some(stdout),
    }
  }

  pub(crate) fn content(content: impl Into<String>) -> Self {
    Self {
      content: Some(content.into()),
      error: None,
      exit_status: None,
      stdout: None,
    }
  }

  pub(crate) fn error(error: impl Display) -> Self {
    Self {
      content: None,
      error: Some(error.to_string()),
      exit_status: None,
      stdout: None,
    }
  }

  pub(crate) fn exit_status(&self) -> Option<i32> {
    self.exit_status
  }

  pub(crate) fn is_error(&self) -> bool {
    self.error.is_some() || self.exit_status.is_some_and(|status| status != 0)
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
      self.error.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n");

    (!output.is_empty()).then_some(output)
  }
}
