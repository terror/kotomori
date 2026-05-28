use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentMessageContent {
  Reasoning(String),
  Text(String),
  ToolCall(ToolInvocation),
}

impl AgentMessageContent {
  pub(crate) fn text(&self) -> Option<&str> {
    match self {
      Self::Reasoning(_) => None,
      Self::Text(text) => Some(text),
      Self::ToolCall(_) => None,
    }
  }
}
