use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum TranscriptEntry {
  Draft(MessageBuffer),
  Error(String),
  Interrupted,
  Message(Message),
  Notice(String),
}

impl From<RunEntry> for TranscriptEntry {
  fn from(entry: RunEntry) -> Self {
    match entry {
      RunEntry::Draft(buffer) => Self::Draft(buffer),
      RunEntry::Message(message) => Self::Message(message),
    }
  }
}
