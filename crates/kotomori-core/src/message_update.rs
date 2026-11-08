use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageUpdate {
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
