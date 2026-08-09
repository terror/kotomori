use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum UserMessageContent {
  Text(String),
  ToolResult { id: String, result: ToolResult },
}

impl UserMessageContent {
  pub(crate) fn text(&self) -> Option<&str> {
    match self {
      Self::Text(text) => Some(text),
      Self::ToolResult { .. } => None,
    }
  }
}
