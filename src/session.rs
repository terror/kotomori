use super::*;

#[derive(Debug)]
pub(crate) struct Session {
  pub(crate) file: SessionFile,
  pub(crate) path: PathBuf,
}

impl Session {
  const VERSION: u32 = 1;

  fn file(options: &Options) -> Result<(PathBuf, SessionFile)> {
    let now = SessionStore::now()?;

    let id = SessionStore::id()?;

    Ok((
      SessionStore::sessions_dir()?.join(format!("{id}.json")),
      SessionFile {
        created_at: now,
        cwd: env::current_dir().context("failed to read current directory")?,
        entries: Vec::new(),
        id,
        model: options.model.to_string(),
        title: None,
        updated_at: now,
        version: Self::VERSION,
      },
    ))
  }

  pub(crate) fn new(options: &Options) -> Result<Self> {
    let (path, file) = Self::file(options)?;

    Ok(Self { file, path })
  }

  pub(crate) fn save(&mut self, transcript: &Transcript) -> Result {
    if transcript.is_empty() && !self.path.exists() {
      return Ok(());
    }

    self.file.entries.clone_from(&transcript.entries);
    self.file.title = Self::title(&transcript.entries);
    self.file.updated_at = SessionStore::now()?;

    SessionStore::write(&self.path, &self.file)
  }

  pub(crate) fn set_model(&mut self, model: &Model) {
    self.file.model = model.to_string();
  }

  pub(crate) fn title(entries: &[TranscriptEntry]) -> Option<String> {
    entries.iter().find_map(|entry| match entry {
      TranscriptEntry::User(content) => {
        let title = SessionStore::compact(content);

        (!title.is_empty()).then_some(title)
      }
      TranscriptEntry::Agent(_)
      | TranscriptEntry::Interrupted
      | TranscriptEntry::Reasoning(_)
      | TranscriptEntry::Tool { .. } => None,
    })
  }
}
