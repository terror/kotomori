use super::*;

#[derive(Debug)]
pub(crate) struct Session {
  pub(crate) database: Database,
  pub(crate) file: SessionFile,
  pub(crate) persisted: bool,
}

impl Session {
  const TITLE_LENGTH: usize = 80;
  const VERSION: u32 = 1;

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    let database = Database::new()?;
    let now = Database::now()?;

    let id = Database::id()?;

    Ok(Self {
      database,
      file: SessionFile {
        created_at: now,
        cwd: env::current_dir().context("failed to read current directory")?,
        entries: Vec::new(),
        id,
        model: settings.model.to_string(),
        title: None,
        updated_at: now,
        version: Self::VERSION,
      },
      persisted: false,
    })
  }

  pub(crate) fn save(&mut self, transcript: &Transcript) -> Result {
    if transcript.is_empty() && !self.persisted {
      return Ok(());
    }

    self.file.entries.clone_from(&transcript.entries);
    self.file.title = Self::title(&transcript.entries);
    self.file.updated_at = Database::now()?;

    self.database.write(&self.file)?;
    self.persisted = true;

    Ok(())
  }

  pub(crate) fn set_model(&mut self, model: &Model) {
    self.file.model = model.to_string();
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
