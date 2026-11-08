use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum TranscriptEntry {
  Draft(MessageBuffer),
  Error(String),
  Interrupted,
  Message(Message),
  Notice(String),
}

impl TranscriptEntry {
  pub(crate) fn into_run_entry(self) -> Option<RunEntry> {
    match self {
      Self::Draft(buffer) => Some(RunEntry::Draft(buffer)),
      Self::Message(message) => Some(RunEntry::Message(message)),
      Self::Error(_) | Self::Interrupted | Self::Notice(_) => None,
    }
  }
}

impl From<RunEntry> for TranscriptEntry {
  fn from(entry: RunEntry) -> Self {
    match entry {
      RunEntry::Draft(buffer) => Self::Draft(buffer),
      RunEntry::Message(message) => Self::Message(message),
    }
  }
}
