use super::*;

#[derive(Debug)]
pub(crate) struct Session {
  pub(crate) created_at: u64,
  pub(crate) cwd: PathBuf,
  pub(crate) entries: Vec<TranscriptEntry>,
  pub(crate) id: String,
  pub(crate) model: String,
  pub(crate) persisted: bool,
  pub(crate) title: Option<String>,
  pub(crate) updated_at: u64,
}

impl Session {
  const TITLE_LENGTH: usize = 80;

  fn id() -> Result<String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, atomic::Ordering::Relaxed);

    Ok(format!(
      "{}-{}-{counter}",
      Self::timestamp()?.as_nanos(),
      process::id()
    ))
  }

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    let now = Self::now()?;

    Ok(Self {
      created_at: now,
      cwd: env::current_dir().context("failed to read current directory")?,
      entries: Vec::new(),
      id: Self::id()?,
      model: settings.model.to_string(),
      persisted: false,
      title: None,
      updated_at: now,
    })
  }

  pub(crate) fn now() -> Result<u64> {
    Ok(Self::timestamp()?.as_secs())
  }

  pub(crate) fn save(
    &mut self,
    database: &Database,
    transcript: &Transcript,
  ) -> Result {
    if transcript.is_empty() && !self.persisted {
      return Ok(());
    }

    self.entries.clone_from(&transcript.entries);
    self.title = Self::title(&transcript.entries);
    self.updated_at = Self::now()?;

    database.add_session(self)?;

    self.persisted = true;

    Ok(())
  }

  pub(crate) fn set_model(&mut self, model: &Model) {
    self.model = model.to_string();
  }

  fn timestamp() -> Result<Duration> {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system clock is before the unix epoch")
  }

  pub(crate) fn title(entries: &[TranscriptEntry]) -> Option<String> {
    entries.iter().find_map(|entry| match entry {
      TranscriptEntry::User(content) => {
        let title = content
          .split_whitespace()
          .collect::<Vec<_>>()
          .join(" ")
          .as_str()
          .truncate(Self::TITLE_LENGTH);

        (!title.is_empty()).then_some(title)
      }
      TranscriptEntry::Agent(_)
      | TranscriptEntry::Error(_)
      | TranscriptEntry::Interrupted
      | TranscriptEntry::Reasoning(_)
      | TranscriptEntry::Tool { .. } => None,
    })
  }
}
