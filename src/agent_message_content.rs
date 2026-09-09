use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
