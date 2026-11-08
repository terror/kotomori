use super::*;

#[derive(Deserialize, Serialize)]
pub(crate) enum SessionEntry {
  Core,
  Error(String),
  Interrupted,
  Notice(String),
}

impl SessionEntry {
  pub(crate) fn restore(
    entries: Vec<Self>,
    snapshot: &SessionSnapshot,
  ) -> Result<Vec<TranscriptEntry>> {
    let mut core = snapshot.entries().iter();

    let transcript = entries
      .into_iter()
      .map(|entry| {
        Ok(match entry {
          Self::Core => core
            .next()
            .context(
              "session transcript has more core entries than its snapshot",
            )?
            .clone()
            .into(),
          Self::Error(error) => TranscriptEntry::Error(error),
          Self::Interrupted => TranscriptEntry::Interrupted,
          Self::Notice(notice) => TranscriptEntry::Notice(notice),
        })
      })
      .collect::<Result<Vec<_>>>()?;

    ensure!(
      core.next().is_none(),
      "session transcript has fewer core entries than its snapshot"
    );

    Ok(transcript)
  }
}

impl From<&TranscriptEntry> for SessionEntry {
  fn from(entry: &TranscriptEntry) -> Self {
    match entry {
      TranscriptEntry::Draft(_) | TranscriptEntry::Message(_) => Self::Core,
      TranscriptEntry::Error(error) => Self::Error(error.clone()),
      TranscriptEntry::Interrupted => Self::Interrupted,
      TranscriptEntry::Notice(notice) => Self::Notice(notice.clone()),
    }
  }
}
