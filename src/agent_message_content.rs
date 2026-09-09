use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum AgentMessageContent {
  Reasoning(String),
  Text(String),
  ToolCall(ToolInvocation),
}
