use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AgentMessageContent {
  Reasoning(String),
  Text(String),
  ToolCall(ToolInvocation),
}

impl AgentMessageContent {
  pub(crate) fn text(&self) -> Option<&str> {
    match self {
      Self::Text(text) => Some(text),
      Self::Reasoning(_) | Self::ToolCall(_) => None,
    }
  }
}
