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
  pub(crate) fn is_agent(&self) -> bool {
    matches!(self, Self::Draft(_) | Self::Message(Message::Agent(_)))
  }

  pub(crate) fn message(&self) -> Option<&Message> {
    match self {
      Self::Message(message) => Some(message),
      Self::Draft(_) | Self::Error(_) | Self::Interrupted | Self::Notice(_) => {
        None
      }
    }
  }
}
