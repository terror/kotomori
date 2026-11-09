use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TranscriptEntry {
  Draft(MessageBuffer),
  Error(String),
  Interrupted,
  Message(Message),
  Notice(String),
}

impl TranscriptEntry {
  const NAMESPACE: &str = "kotomori.tui";

  pub(crate) fn from_session_entry(entry: &SessionEntry) -> Option<Self> {
    match entry {
      SessionEntry::Run(entry) => Some(entry.clone().into()),
      SessionEntry::Custom(entry)
        if entry.namespace == Self::NAMESPACE && entry.version == 1 =>
      {
        match entry.kind.as_str() {
          "error" => entry.data.as_str().map(|error| Self::Error(error.into())),
          "interrupted" if entry.data.is_null() => Some(Self::Interrupted),
          "notice" => entry
            .data
            .as_str()
            .map(|notice| Self::Notice(notice.into())),
          _ => None,
        }
      }
      SessionEntry::Custom(_) => None,
    }
  }

  pub(crate) fn into_session_entry(self) -> SessionEntry {
    let (kind, data) = match self {
      Self::Draft(buffer) => return RunEntry::Draft(buffer).into(),
      Self::Message(message) => return RunEntry::Message(message).into(),
      Self::Error(error) => ("error", serde_json::Value::String(error)),
      Self::Interrupted => ("interrupted", serde_json::Value::Null),
      Self::Notice(notice) => ("notice", serde_json::Value::String(notice)),
    };

    CustomEntry {
      data,
      kind: kind.into(),
      namespace: Self::NAMESPACE.into(),
      version: 1,
    }
    .into()
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
