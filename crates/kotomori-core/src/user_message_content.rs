use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum UserMessageContent {
  Text(String),
  ToolResult {
    call: CallId,
    name: ToolName,
    result: ToolResult,
  },
}

impl UserMessageContent {
  #[must_use]
  pub fn is_tool_result_for(&self, call: &CallId) -> bool {
    matches!(self, Self::ToolResult { call: id, .. } if id == call)
  }

  #[must_use]
  pub fn text(&self) -> Option<&str> {
    match self {
      Self::Text(text) => Some(text),
      Self::ToolResult { .. } => None,
    }
  }
}
