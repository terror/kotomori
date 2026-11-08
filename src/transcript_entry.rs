use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum TranscriptEntry {
  Draft(MessageBuffer),
  Error(String),
  Interrupted,
  Message(Message),
  Notice(String),
}
