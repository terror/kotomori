use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum MessageDraft {
  Content(AssistantContent),
  Reasoning(String),
}
