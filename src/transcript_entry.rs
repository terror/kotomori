use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum TranscriptEntry {
  Error(String),
  Interrupted,
  Message(Message),
  Notice(String),
}

impl TranscriptEntry {
  pub(crate) fn message(&self) -> Option<&Message> {
    match self {
      Self::Message(message) => Some(message),
      Self::Error(_) | Self::Interrupted | Self::Notice(_) => None,
    }
  }
}
