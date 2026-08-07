use super::*;

#[derive(Debug)]
pub(crate) struct Database {
  connection: Connection,
}

impl Database {
  const DATABASE_NAME: &str = "kotomori.db";
  const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_initial.sql")];
  const SCHEMA_VERSION: usize = Self::MIGRATIONS.len();

  pub(crate) fn get_sessions(&self) -> Result<Vec<Session>> {
    let directory =
      env::current_dir().context("failed to read current directory")?;

    let directory = directory
      .to_str()
      .context("current directory is not valid UTF-8")?;

    let mut statement = self.connection.prepare(
      "SELECT id, updated_at, directory, model, title
       FROM sessions
       WHERE directory = ?1
       ORDER BY updated_at DESC, id DESC",
    )?;

    let rows = statement.query_map([directory], |row| {
      Ok(Session {
        created_at: 0,
        directory: row.get::<_, String>(2)?.into(),
        entries: Vec::new(),
        id: Some(row.get(0)?),
        model: row.get(3)?,
        title: row.get(4)?,
        updated_at: row.get_u64(1)?,
      })
    })?;

    rows.collect::<rusqlite::Result<_>>().map_err(Into::into)
  }

  pub(crate) fn load_session(&self, id: i64) -> Result<Session> {
    self
      .connection
      .query_row(
        "SELECT id, created_at, updated_at, directory, model, title, entries
         FROM sessions
         WHERE id = ?1",
        [id],
        |row| {
          Ok(Session {
            created_at: row.get_u64(1)?,
            directory: row.get::<_, String>(3)?.into(),
            entries: serde_json::from_str(&row.get::<_, String>(6)?).map_err(
              |error| {
                rusqlite::Error::FromSqlConversionFailure(
                  6,
                  rusqlite::types::Type::Text,
                  Box::new(error),
                )
              },
            )?,
            id: Some(row.get(0)?),
            model: row.get(4)?,
            title: row.get(5)?,
            updated_at: row.get_u64(2)?,
          })
        },
      )
      .with_context(|| format!("failed to load session `{id}`"))
  }

  pub(crate) fn new() -> Result<Self> {
    if cfg!(test) {
      Self::try_from(Connection::open_in_memory()?)
    } else {
      let root = if let Some(path) = env::var_os("KOTOMORI_HOME") {
        PathBuf::from(path)
      } else if let Some(path) = env::var_os("XDG_STATE_HOME") {
        PathBuf::from(path).join("kotomori")
      } else {
        let Some(home) = env::var_os("HOME") else {
          bail!("HOME is not set");
        };

        PathBuf::from(home).join(".local/state/kotomori")
      };

      fs::create_dir_all(&root).with_context(|| {
        format!("failed to create state directory {}", root.display())
      })?;

      Self::try_from(root.join(Self::DATABASE_NAME).as_path())
    }
  }

  pub(crate) fn save_session(&self, session: &mut Session) -> Result {
    let directory = session
      .directory
      .to_str()
      .context("session directory is not valid UTF-8")?;

    let entries = serde_json::to_string(&session.entries)
      .context("failed to serialize session transcript")?;

    let created_at = i64::try_from(session.created_at)
      .context("session creation time exceeds SQLite integer range")?;

    let updated_at = i64::try_from(session.updated_at)
      .context("session update time exceeds SQLite integer range")?;

    if let Some(id) = session.id {
      let updated = self.connection.execute(
        "UPDATE sessions SET
           updated_at = ?1,
           directory = ?2,
           model = ?3,
           title = ?4,
           entries = ?5
         WHERE id = ?6",
        params![
          updated_at,
          directory,
          session.model,
          session.title,
          entries,
          id,
        ],
      )?;

      if updated == 0 {
        bail!("session `{id}` no longer exists");
      }
    } else {
      session.id = Some(self.connection.query_row(
        "INSERT INTO sessions (
           created_at, updated_at, directory, model, title, entries
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         RETURNING id",
        params![
          created_at,
          updated_at,
          directory,
          session.model,
          session.title,
          entries,
        ],
        |row| row.get(0),
      )?);
    }

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
  fn save_session_assigns_and_reuses_integer_id() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    let mut session = Session {
      created_at: 0,
      directory: env::current_dir().unwrap(),
      entries: Vec::new(),
      id: None,
      model: "mock:local".into(),
      title: None,
      updated_at: 0,
    };

    database.save_session(&mut session).unwrap();

    assert_eq!(session.id, Some(1));

    database.save_session(&mut session).unwrap();

    assert_eq!(
      database
        .connection
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| {
          row.get::<_, u32>(0)
        })
        .unwrap(),
      1,
    );
  }

  #[test]
  fn summaries_do_not_decode_transcripts() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    let directory = env::current_dir().unwrap();

    database
      .connection
      .execute(
        "INSERT INTO sessions (
           created_at, updated_at, directory, model, title, entries
         ) VALUES (0, 0, ?1, 'mock:local', NULL, '{}')",
        [directory.to_str().unwrap()],
      )
      .unwrap();

    assert_eq!(database.get_sessions().unwrap().len(), 1);

    assert_eq!(
      database
        .load_session(1)
        .unwrap_err()
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
      [
        "failed to load session `1`",
        "Conversion error from type Text at index: 6, invalid type: map, expected a sequence at line 1 column 0",
        "invalid type: map, expected a sequence at line 1 column 0",
      ],
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
