use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(
  try_from = "VersionedSessionSnapshot",
  into = "VersionedSessionSnapshot"
)]
pub struct SessionSnapshot {
  entries: Vec<RunEntry>,
}

impl SessionSnapshot {
  pub const VERSION: u32 = 1;

  #[must_use]
  pub fn conversation(&self) -> Conversation {
    self
      .entries
      .iter()
      .cloned()
      .filter_map(RunEntry::into_message)
      .collect::<Vec<_>>()
      .into()
  }

  #[must_use]
  pub fn entries(&self) -> &[RunEntry] {
    &self.entries
  }

  pub fn new(entries: Vec<RunEntry>) -> Result<Self> {
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
  entries: Vec<RunEntry>,
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
