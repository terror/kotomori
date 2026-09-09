use super::*;

#[derive(Debug)]
pub(crate) struct Session {
  pub(crate) created_at: u64,
  pub(crate) directory: PathBuf,
  pub(crate) id: Option<i64>,
  pub(crate) model: String,
  pub(crate) title: Option<String>,
  pub(crate) transcript: Transcript,
  pub(crate) updated_at: u64,
}

impl Session {
  const TITLE_LENGTH: usize = 80;

  fn age(&self) -> String {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
      return "unknown age".into();
    };

    let seconds = now.as_secs().saturating_sub(self.updated_at);

    match seconds {
      0..=59 => "now".into(),
      60..=3_599 => format!("{}m ago", seconds / 60),
      3_600..=86_399 => format!("{}h ago", seconds / 3_600),
      _ => format!("{}d ago", seconds / 86_400),
    }
  }

  pub(crate) fn detail(&self) -> String {
    format!(
      "{} · {} · {}",
      self.model,
      DirectoryDisplay::new(&self.directory),
      self.age()
    )
  }

  pub(crate) fn matches(&self, query: &str) -> bool {
    let search = format!(
      "{} {} {} {}",
      self.title.as_deref().unwrap_or("Untitled session"),
      self.model,
      DirectoryDisplay::new(&self.directory),
      self.id.map_or_else(String::new, |id| id.to_string()),
    )
    .chars()
    .flat_map(char::to_lowercase)
    .collect::<String>();

    query.split_whitespace().all(|term| {
      search.contains(
        &term
          .chars()
          .flat_map(char::to_lowercase)
          .collect::<String>(),
      )
    })
  }

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
      model: settings.model.to_string(),
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

  pub(crate) fn set_model(&mut self, model: &Model) {
    self.model = model.to_string();
  }
}
