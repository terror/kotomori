use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MessageUpdate {
  Complete(AgentMessage),
  Content {
    index: usize,
    content: AssistantContent,
  },
  MessageId(String),
  ReasoningDelta {
    delta: String,
    index: usize,
  },
  Text {
    delta: String,
    index: usize,
  },
}
