use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(
  try_from = "VersionedSessionSnapshot",
  into = "VersionedSessionSnapshot"
)]
pub struct SessionSnapshot {
  entries: Vec<SessionEntry>,
}

impl SessionSnapshot {
  pub const VERSION: u32 = 2;

  #[must_use]
  pub fn conversation(&self) -> Conversation {
    self
      .entries
      .iter()
      .cloned()
      .filter_map(SessionEntry::into_message)
      .collect::<Vec<_>>()
      .into()
  }

  #[must_use]
  pub fn entries(&self) -> &[SessionEntry] {
    &self.entries
  }

  #[must_use]
  pub fn into_entries(self) -> Vec<SessionEntry> {
    self.entries
  }

  pub fn new(entries: Vec<SessionEntry>) -> Result<Self> {
    let snapshot = Self { entries };

    snapshot.conversation().validate()?;

    Ok(snapshot)
  }
}

impl TryFrom<VersionedSessionSnapshot> for SessionSnapshot {
  type Error = Error;

  fn try_from(snapshot: VersionedSessionSnapshot) -> Result<Self> {
    ensure!(
      snapshot.version == Self::VERSION,
      "session snapshot version {} is unsupported; expected {}",
      snapshot.version,
      Self::VERSION,
    );

    Self::new(snapshot.entries)
  }
}

#[derive(Deserialize, Serialize)]
struct VersionedSessionSnapshot {
  entries: Vec<SessionEntry>,
  version: u32,
}

impl From<SessionSnapshot> for VersionedSessionSnapshot {
  fn from(snapshot: SessionSnapshot) -> Self {
    Self {
      entries: snapshot.entries,
      version: SessionSnapshot::VERSION,
    }
  }
}
