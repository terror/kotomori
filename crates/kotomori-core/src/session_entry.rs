use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SessionEntry {
  Custom(CustomEntry),
  Run(RunEntry),
}

impl SessionEntry {
  #[must_use]
  pub fn into_message(self) -> Option<Message> {
    match self {
      Self::Custom(_) => None,
      Self::Run(entry) => entry.into_message(),
    }
  }
}

impl From<CustomEntry> for SessionEntry {
  fn from(entry: CustomEntry) -> Self {
    Self::Custom(entry)
  }
}

impl From<RunEntry> for SessionEntry {
  fn from(entry: RunEntry) -> Self {
    Self::Run(entry)
  }
}
