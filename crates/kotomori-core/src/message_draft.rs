use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MessageDraft {
  Content(AssistantContent),
  Reasoning(String),
}
