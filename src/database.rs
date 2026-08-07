use super::*;

#[derive(Debug)]
pub(crate) struct Database {
  connection: Connection,
}

impl Database {
  const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_initial.sql")];

  const SCHEMA_VERSION: usize = Self::MIGRATIONS.len();

  pub(crate) fn id() -> Result<String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, atomic::Ordering::Relaxed);

    Ok(format!(
      "{}-{}-{counter}",
      Self::timestamp()?.as_nanos(),
      process::id()
    ))
  }

  pub(crate) fn list(&self) -> Result<Vec<SessionSummary>> {
    let cwd = env::current_dir().context("failed to read current directory")?;

    let cwd = cwd
      .to_str()
      .context("current directory is not valid UTF-8")?;

    let mut statement = self.connection.prepare(
      "SELECT id, version, created_at, updated_at, cwd, model, title, entries
       FROM sessions
       WHERE cwd = ?1
       ORDER BY updated_at DESC, id DESC",
    )?;

    let rows = statement.query_map([cwd], |row| {
      Ok(SessionFile {
        id: row.get(0)?,
        version: row.get(1)?,
        created_at: Self::read_u64(row, 2)?,
        updated_at: Self::read_u64(row, 3)?,
        cwd: row.get::<_, String>(4)?.into(),
        model: row.get(5)?,
        title: row.get(6)?,
        entries: serde_json::from_str(&row.get::<_, String>(7)?).map_err(
          |error| {
            rusqlite::Error::FromSqlConversionFailure(
              7,
              rusqlite::types::Type::Text,
              Box::new(error),
            )
          },
        )?,
      })
    })?;

    rows
      .map(|row| {
        let file = row?;
        Ok(SessionSummary::new(file.id.clone(), file))
      })
      .collect()
  }

  pub(crate) fn load(self, id: &str) -> Result<Session> {
    let file = self
      .connection
      .query_row(
        "SELECT id, version, created_at, updated_at, cwd, model, title, entries
         FROM sessions
         WHERE id = ?1",
        [id],
        |row| {
          Ok(SessionFile {
            id: row.get(0)?,
            version: row.get(1)?,
            created_at: Self::read_u64(row, 2)?,
            updated_at: Self::read_u64(row, 3)?,
            cwd: row.get::<_, String>(4)?.into(),
            model: row.get(5)?,
            title: row.get(6)?,
            entries: serde_json::from_str(&row.get::<_, String>(7)?).map_err(
              |error| {
                rusqlite::Error::FromSqlConversionFailure(
                  7,
                  rusqlite::types::Type::Text,
                  Box::new(error),
                )
              },
            )?,
          })
        },
      )
      .with_context(|| format!("failed to load session `{id}`"))?;

    Ok(Session {
      database: self,
      file,
      persisted: true,
    })
  }

  pub(crate) fn new() -> Result<Self> {
    let root = Self::root()?;

    fs::create_dir_all(&root).with_context(|| {
      format!("failed to create state directory {}", root.display())
    })?;

    Self::try_from(root.join("sessions.db").as_path())
  }

  pub(crate) fn now() -> Result<u64> {
    Ok(Self::timestamp()?.as_secs())
  }

  fn read_u64(row: &rusqlite::Row, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;

    u64::try_from(value)
      .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
  }

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

  fn timestamp() -> Result<Duration> {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system clock is before the unix epoch")
  }

  pub(crate) fn write(&self, file: &SessionFile) -> Result {
    let cwd = file
      .cwd
      .to_str()
      .context("session directory is not valid UTF-8")?;

    let entries = serde_json::to_string(&file.entries)
      .context("failed to serialize session transcript")?;

    let created_at = i64::try_from(file.created_at)
      .context("session creation time exceeds SQLite integer range")?;

    let updated_at = i64::try_from(file.updated_at)
      .context("session update time exceeds SQLite integer range")?;

    self.connection.execute(
      "INSERT INTO sessions (
         id, version, created_at, updated_at, cwd, model, title, entries
       ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
       ON CONFLICT (id) DO UPDATE SET
         version = excluded.version,
         updated_at = excluded.updated_at,
         cwd = excluded.cwd,
         model = excluded.model,
         title = excluded.title,
         entries = excluded.entries",
      params![
        file.id,
        file.version,
        created_at,
        updated_at,
        cwd,
        file.model,
        file.title,
        entries,
      ],
    )?;

    Ok(())
  }
}

impl TryFrom<Connection> for Database {
  type Error = Error;

  fn try_from(mut connection: Connection) -> Result<Self> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "journal_mode", "WAL")?;

    let transaction =
      connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 =
      transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    let Ok(version) = usize::try_from(version) else {
      bail!(
        "database schema version {version} is unsupported; expected {}",
        Self::SCHEMA_VERSION,
      );
    };

    if version > Self::SCHEMA_VERSION {
      bail!(
        "database schema version {version} is unsupported; expected {}",
        Self::SCHEMA_VERSION,
      );
    }

    for (version, migration) in
      Self::MIGRATIONS.iter().enumerate().skip(version)
    {
      let version = version + 1;

      transaction.execute_batch(migration).with_context(|| {
        format!("failed to apply database migration {version}")
      })?;
      transaction.pragma_update(
        None,
        "user_version",
        i64::try_from(version)?,
      )?;
    }

    transaction.commit()?;

    Ok(Self { connection })
  }
}

impl TryFrom<&Path> for Database {
  type Error = Error;

  fn try_from(path: &Path) -> Result<Self> {
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;

      let directory = path
        .parent()
        .context("database path has no parent directory")?;

      fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    }

    let database =
      Self::try_from(Connection::open(path)?).with_context(|| {
        format!("failed to open database `{}`", path.display())
      })?;

    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;

      fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(database)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn migrations_create_schema() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    assert_eq!(
      database
        .connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .unwrap(),
      i64::try_from(Database::SCHEMA_VERSION).unwrap(),
    );
  }

  #[test]
  fn unsupported_schema_is_rejected() {
    let connection = Connection::open_in_memory().unwrap();

    connection.execute_batch("PRAGMA user_version = 2").unwrap();

    assert_eq!(
      Database::try_from(connection).unwrap_err().to_string(),
      "database schema version 2 is unsupported; expected 1",
    );
  }
}
