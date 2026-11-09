use super::*;

#[derive(Debug)]
pub(crate) struct Database {
  connection: Connection,
}

impl Database {
  const SCHEMA_VERSION: i64 = 2;

  pub(crate) fn fork_session(
    &self,
    session: &Session,
    now: u64,
  ) -> Result<Session> {
    let transaction = self.connection.unchecked_transaction()?;

    self.save_session(session, now)?;

    let mut session = Session {
      core: session.core.fork()?,
      title: session.title.clone(),
      ..Session::new(&session.settings, now)
    };

    session.refresh();

    let saved = self.save_session(&session, now)?;

    transaction.commit()?;

    Ok(Session {
      id: Some(saved.id),
      title: saved.title,
      ..session
    })
  }

  pub(crate) fn get_sessions(
    &self,
    directory: &Path,
  ) -> Result<Vec<SessionSummary>> {
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
      Ok(SessionSummary {
        directory: row.get::<_, String>(2)?.into(),
        id: row.get(0)?,
        model: row.get::<_, DatabaseModel>(3)?.0,
        title: row.get(4)?,
        updated_at: row.get(1)?,
      })
    })?;

    rows.collect::<rusqlite::Result<_>>().map_err(Into::into)
  }

  pub(crate) fn load_session(
    &self,
    id: i64,
    settings: &Settings,
  ) -> Result<Session> {
    self
      .connection
      .query_row(
        "SELECT id, created_at, updated_at, directory, model, title, snapshot
         FROM sessions
         WHERE id = ?1",
        [id],
        |row| {
          let snapshot =
            serde_json::from_str::<SessionSnapshot>(&row.get::<_, String>(6)?)
              .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                  6,
                  rusqlite::types::Type::Text,
                  Box::new(error),
                )
              })?;

          let mut session = Session {
            core: snapshot.into(),
            created_at: row.get(1)?,
            id: Some(row.get(0)?),
            settings: Settings {
              directory: row.get::<_, String>(3)?.into(),
              model: row.get::<_, DatabaseModel>(4)?.0,
              ..settings.clone()
            },
            title: row.get(5)?,
            transcript: Vec::new(),
            updated_at: row.get(2)?,
          };

          session.refresh();

          Ok(session)
        },
      )
      .with_context(|| format!("failed to load session `{id}`"))
  }

  pub(crate) fn new(path: &Path) -> Result<Self> {
    let directory = path
      .parent()
      .context("database path has no parent directory")?;

    fs::create_dir_all(directory).with_context(|| {
      format!("failed to create state directory {}", directory.display())
    })?;

    Self::try_from(path)
  }

  pub(crate) fn save_session(
    &self,
    session: &Session,
    updated_at: u64,
  ) -> Result<SavedSession> {
    let snapshot = serde_json::to_string(&session.snapshot()?)
      .context("failed to serialize session snapshot")?;

    let directory = session
      .settings
      .directory
      .to_str()
      .context("session directory is not valid UTF-8")?;

    let title = session.title.clone().or_else(|| session.generate_title());

    let id = if let Some(id) = session.id {
      let updated = self.connection.execute(
        "UPDATE sessions SET
           updated_at = ?1,
           directory = ?2,
           model = ?3,
           title = ?4,
           snapshot = ?5
         WHERE id = ?6",
        params![
          updated_at,
          directory,
          session.settings.model.to_string(),
          title,
          snapshot,
          id,
        ],
      )?;

      if updated == 0 {
        bail!("session `{id}` no longer exists");
      }

      id
    } else {
      self.connection.query_row(
        "INSERT INTO sessions (
           created_at, updated_at, directory, model, title, snapshot
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         RETURNING id",
        params![
          session.created_at,
          updated_at,
          directory,
          session.settings.model.to_string(),
          title,
          snapshot,
        ],
        |row| row.get(0),
      )?
    };

    Ok(SavedSession {
      id,
      title,
      updated_at,
    })
  }
}

impl SessionStore for Database {
  type Key = i64;

  fn load(&self, id: &i64) -> Result<Option<SessionSnapshot>> {
    self
      .connection
      .query_row("SELECT snapshot FROM sessions WHERE id = ?1", [id], |row| {
        serde_json::from_str(&row.get::<_, String>(0)?).map_err(|error| {
          rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(error),
          )
        })
      })
      .optional()
      .with_context(|| format!("failed to load session snapshot `{id}`"))
  }

  fn save(&self, id: &i64, snapshot: &SessionSnapshot) -> Result {
    let snapshot = serde_json::to_string(snapshot)
      .context("failed to serialize session snapshot")?;

    let updated = self.connection.execute(
      "UPDATE sessions SET snapshot = ?1, updated_at = ?2 WHERE id = ?3",
      params![snapshot, now()?, id],
    )?;

    ensure!(updated != 0, "session `{id}` no longer exists");

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

    if version == 0 {
      transaction
        .execute_batch(include_str!("../migrations/0002_initial.sql"))
        .context("failed to initialize database schema")?;

      transaction.pragma_update(None, "user_version", Self::SCHEMA_VERSION)?;
    } else {
      ensure!(
        version == Self::SCHEMA_VERSION,
        "database schema version {version} is unsupported; expected {}",
        Self::SCHEMA_VERSION,
      );
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
  fn save_session_rejects_out_of_range_timestamps() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    let timestamp = u64::try_from(i64::MAX).unwrap() + 1;

    for (id, created_at, updated_at) in [
      (None, timestamp, 0),
      (None, 0, timestamp),
      (Some(1), 0, timestamp),
    ] {
      let session = Session {
        core: kotomori_core::Session::default(),
        created_at,
        id,
        settings: Settings {
          directory: "foo".into(),
          max_output_tokens: None,
          max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
          model: "mock:foo".parse().unwrap(),
          prompt: None,
          yolo: false,
        },
        title: None,
        transcript: Vec::new(),
        updated_at,
      };

      assert_matches!(
        database
          .save_session(&session, updated_at)
          .unwrap_err()
          .downcast::<rusqlite::Error>()
          .unwrap(),
        rusqlite::Error::ToSqlConversionFailure(error)
          if error.is::<std::num::TryFromIntError>(),
      );

      assert_eq!(session.id, id);
    }
  }

  #[test]
  fn save_session_round_trips() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    let timestamp = u64::try_from(i64::MAX).unwrap();

    let session = Session {
      core: kotomori_core::Session::default(),
      created_at: timestamp,
      id: None,
      settings: Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:foo".parse().unwrap(),
        prompt: Some("bar".into()),
        yolo: true,
      },
      title: None,
      transcript: Vec::new(),
      updated_at: 0,
    };

    assert_eq!(
      database.save_session(&session, 0).unwrap(),
      SavedSession {
        id: 1,
        title: None,
        updated_at: 0,
      },
    );

    assert_eq!(
      database
        .load_session(1, &session.settings)
        .unwrap()
        .settings,
      session.settings,
    );

    let mut session = Session {
      id: Some(1),
      settings: Settings {
        model: "mock:bar".parse().unwrap(),
        ..session.settings
      },
      ..session
    };

    session.push(TranscriptEntry::Message(Message::User(vec![
      UserMessageContent::Text("foo".into()),
    ])));

    assert_eq!(
      database.save_session(&session, timestamp).unwrap(),
      SavedSession {
        id: 1,
        title: Some("foo".into()),
        updated_at: timestamp,
      },
    );

    let settings = Settings {
      directory: "bar".into(),
      max_output_tokens: None,
      max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
      model: "mock:baz".parse().unwrap(),
      prompt: Some("qux".into()),
      yolo: false,
    };

    let loaded = database.load_session(1, &settings).unwrap();

    assert_eq!(
      loaded.settings,
      Settings {
        directory: "foo".into(),
        model: "mock:bar".parse().unwrap(),
        ..settings.clone()
      },
    );

    assert_eq!(
      (loaded.created_at, loaded.updated_at),
      (timestamp, timestamp),
    );

    assert_eq!(
      database.get_sessions(Path::new("foo")).unwrap(),
      [SessionSummary {
        directory: "foo".into(),
        id: 1,
        model: "mock:bar".parse().unwrap(),
        title: Some("foo".into()),
        updated_at: timestamp,
      }],
    );

    let snapshot =
      SessionSnapshot::new(vec![SessionEntry::Custom(CustomEntry {
        data: serde_json::json!({"foo": "bar"}),
        kind: "baz".into(),
        namespace: "qux".into(),
        version: 42,
      })])
      .unwrap();

    SessionStore::save(&database, &1, &snapshot).unwrap();

    let loaded = database.load_session(1, &settings).unwrap();

    assert_eq!(loaded.snapshot().unwrap(), snapshot);
    assert_eq!(loaded.transcript, []);
    assert_eq!(SessionStore::load(&database, &1).unwrap(), Some(snapshot));
  }

  #[test]
  fn session_reads_reject_invalid_models() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    let settings = Settings {
      directory: "foo".into(),
      max_output_tokens: None,
      max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    database
      .connection
      .execute(
        r#"INSERT INTO sessions (
           created_at, updated_at, directory, model, snapshot
         ) VALUES (0, 0, ?1, 'foo', '{"version":2,"entries":[]}')"#,
        ["foo"],
      )
      .unwrap();

    assert_eq!(
      format!("{:#}", database.load_session(1, &settings).unwrap_err()),
      "failed to load session `1`: Conversion error from type Text at index: 4, model must be PROVIDER:MODEL: model must be PROVIDER:MODEL",
    );

    assert_eq!(
      database
        .get_sessions(Path::new("foo"))
        .unwrap_err()
        .to_string(),
      "Conversion error from type Text at index: 3, model must be PROVIDER:MODEL",
    );
  }

  #[test]
  fn session_reads_reject_negative_timestamps() {
    let settings = Settings {
      directory: "foo".into(),
      max_output_tokens: None,
      max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    for (created_at, updated_at, index) in [(-1, 0, 1), (0, -1, 2)] {
      let database =
        Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

      database
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();

      database
        .connection
        .execute(
          r#"INSERT INTO sessions (
             created_at, updated_at, directory, model, snapshot
           ) VALUES (?1, ?2, ?3, 'mock:foo', '{"version":2,"entries":[]}')"#,
          params![created_at, updated_at, "foo"],
        )
        .unwrap();

      assert_eq!(
        format!("{:#}", database.load_session(1, &settings).unwrap_err()),
        format!(
          "failed to load session `1`: Integer -1 out of range at index {index}"
        ),
      );

      if updated_at < 0 {
        assert_eq!(
          database
            .get_sessions(Path::new("foo"))
            .unwrap_err()
            .to_string(),
          "Integer -1 out of range at index 1",
        );
      }
    }
  }
}
