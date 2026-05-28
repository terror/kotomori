use super::*;

#[derive(Debug)]
pub(crate) struct SessionStore;

impl SessionStore {
  pub(crate) fn compact(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut chars = text.chars();

    let title = chars.by_ref().take(80).collect::<String>();

    if chars.next().is_some() {
      format!("{title}...")
    } else {
      title
    }
  }

  pub(crate) fn display_directory(path: &Path) -> String {
    match env::var_os("HOME").map(PathBuf::from) {
      Some(home) => match path.strip_prefix(home) {
        Ok(relative) if relative.as_os_str().is_empty() => "~".to_string(),
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => path.display().to_string(),
      },
      None => path.display().to_string(),
    }
  }

  pub(crate) fn id() -> Result<String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, atomic::Ordering::Relaxed);

    Ok(format!(
      "{}-{}-{counter}",
      Self::timestamp()?.as_nanos(),
      process::id()
    ))
  }

  pub(crate) fn list() -> Result<Vec<SessionSummary>> {
    let cwd = env::current_dir().context("failed to read current directory")?;
    let directory = Self::sessions_dir()?;

    let entries = match fs::read_dir(&directory) {
      Ok(entries) => entries,
      Err(error) if error.kind() == io::ErrorKind::NotFound => {
        return Ok(Vec::new());
      }
      Err(error) => {
        return Err(error).with_context(|| {
          format!("failed to read sessions in {}", directory.display())
        });
      }
    };

    let mut summaries = Vec::new();

    for entry in entries {
      let entry = entry.with_context(|| {
        format!("failed to read sessions in {}", directory.display())
      })?;

      let path = entry.path();

      if path
        .extension()
        .is_none_or(|extension| extension != OsStr::new("json"))
      {
        continue;
      }

      let Ok(file) = Self::load_file(&path) else {
        continue;
      };

      if file.cwd != cwd {
        continue;
      }

      summaries.push(SessionSummary::new(path, file));
    }

    summaries.sort_by_key(|summary| Reverse(summary.updated_at));

    Ok(summaries)
  }

  pub(crate) fn load(path: &Path) -> Result<Session> {
    Ok(Session {
      file: Self::load_file(path)?,
      path: path.to_owned(),
    })
  }

  fn load_file(path: &Path) -> Result<SessionFile> {
    let bytes = fs::read(path)
      .with_context(|| format!("failed to read session {}", path.display()))?;

    serde_json::from_slice(&bytes)
      .with_context(|| format!("failed to parse session {}", path.display()))
  }

  pub(crate) fn now() -> Result<u64> {
    Ok(Self::timestamp()?.as_secs())
  }

  #[cfg(not(test))]
  fn root() -> Result<PathBuf> {
    if let Some(path) = env::var_os("KOTOMORI_HOME") {
      Ok(PathBuf::from(path))
    } else if let Some(path) = env::var_os("XDG_STATE_HOME") {
      Ok(PathBuf::from(path).join("kotomori"))
    } else {
      let Some(home) = env::var_os("HOME") else {
        bail!("HOME is not set");
      };

      Ok(PathBuf::from(home).join(".local/state/kotomori"))
    }
  }

  #[cfg(test)]
  fn root() -> Result<PathBuf> {
    if let Some(path) = env::var_os("KOTOMORI_HOME") {
      Ok(PathBuf::from(path))
    } else {
      let _ = env::current_dir().context("failed to read current directory")?;

      Ok(env::temp_dir().join(format!("kotomori-{}", process::id())))
    }
  }

  pub(crate) fn sessions_dir() -> Result<PathBuf> {
    Ok(Self::root()?.join("sessions"))
  }

  fn timestamp() -> Result<Duration> {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system clock is before the unix epoch")
  }

  pub(crate) fn write(path: &Path, file: &SessionFile) -> Result {
    let parent = path.parent().with_context(|| {
      format!("session path has no parent: {}", path.display())
    })?;

    fs::create_dir_all(parent).with_context(|| {
      format!("failed to create session directory {}", parent.display())
    })?;

    let mut bytes =
      serde_json::to_vec_pretty(file).context("failed to serialize session")?;

    bytes.push(b'\n');

    let temporary = path.with_extension("json.tmp");

    fs::write(&temporary, bytes).with_context(|| {
      format!("failed to write session {}", temporary.display())
    })?;

    fs::rename(&temporary, path)
      .with_context(|| format!("failed to save session {}", path.display()))
  }
}
