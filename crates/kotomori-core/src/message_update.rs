use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
