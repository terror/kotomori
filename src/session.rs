use super::*;

#[derive(Debug)]
pub(crate) struct Session {
  pub(crate) created_at: u64,
  pub(crate) directory: PathBuf,
  pub(crate) id: Option<i64>,
  pub(crate) settings: Settings,
  pub(crate) title: Option<String>,
  pub(crate) transcript: Transcript,
  pub(crate) updated_at: u64,
}

impl Session {
  const TITLE_LENGTH: usize = 80;

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system clock is before the unix epoch")?
      .as_secs();

    Ok(Self {
      created_at: now,
      directory: env::current_dir()
        .context("failed to read current directory")?,
      id: None,
      settings: settings.clone(),
      title: None,
      transcript: Transcript::default(),
      updated_at: now,
    })
  }

  pub(crate) fn save(&mut self, database: &Database) -> Result {
    if self.transcript.is_empty() && self.id.is_none() {
      return Ok(());
    }

    self.title = self
      .transcript
      .entries
      .iter()
      .filter_map(TranscriptEntry::message)
      .filter_map(Message::user_content)
      .find_map(|content| {
        let title = content
          .split_whitespace()
          .collect::<Vec<_>>()
          .join(" ")
          .as_str()
          .truncate(Self::TITLE_LENGTH);

        (!title.is_empty()).then_some(title)
      });

    self.updated_at = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system clock is before the unix epoch")?
      .as_secs();

    database.save_session(self)?;

    Ok(())
  }
}
