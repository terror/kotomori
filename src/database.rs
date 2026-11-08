use super::*;

#[derive(Debug)]
pub(crate) struct Database {
  connection: Connection,
}

impl Database {
  const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_session_snapshots.sql"),
  ];
  const SCHEMA_VERSION: usize = Self::MIGRATIONS.len();

  pub(crate) fn fork_session(
    &self,
    session: &Session,
    now: u64,
  ) -> Result<Session> {
    let transaction = self.connection.unchecked_transaction()?;

    self.save_session_inner(session, now)?;

    let session = Session {
      conversation: session.conversation.clone(),
      title: session.title.clone(),
      transcript: session.transcript.clone(),
      ..Session::new(&session.settings, now)
    };

    let saved = self.save_session_inner(&session, now)?;

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
        "SELECT id, created_at, updated_at, directory, model, title, entries, snapshot
         FROM sessions
         WHERE id = ?1",
        [id],
        |row| {
          let entries = row.get::<_, String>(6)?;
          let snapshot = row.get::<_, Option<String>>(7)?;

          let core = Self::snapshot(&entries, snapshot.as_deref())
            .map_err(|error| {
              rusqlite::Error::FromSqlConversionFailure(
                if snapshot.is_some() { 7 } else { 6 },
                rusqlite::types::Type::Text,
                error.into(),
              )
            })?;

          let transcript = if snapshot.is_some() {
            serde_json::from_str::<Vec<SessionEntry>>(&entries)
              .map_err(Error::from)
              .and_then(|entries| SessionEntry::restore(entries, &core))
          } else {
            serde_json::from_str::<Vec<TranscriptEntry>>(&entries)
              .map_err(Error::from)
          }
          .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
              6,
              rusqlite::types::Type::Text,
              error.into(),
            )
          })?;

          Ok(Session {
            conversation: core.conversation(),
            created_at: row.get(1)?,
            id: Some(row.get(0)?),
            settings: Settings {
              directory: row.get::<_, String>(3)?.into(),
              model: row.get::<_, DatabaseModel>(4)?.0,
              ..settings.clone()
            },
            title: row.get(5)?,
            transcript,
            updated_at: row.get(2)?,
          })
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
    let transaction = self.connection.unchecked_transaction()?;

    let saved = self.save_session_inner(session, updated_at)?;

    transaction.commit()?;

    Ok(saved)
  }

  fn save_session_inner(
    &self,
    session: &Session,
    updated_at: u64,
  ) -> Result<SavedSession> {
    let snapshot = session.snapshot()?;

    let directory = session
      .settings
      .directory
      .to_str()
      .context("session directory is not valid UTF-8")?;

    let entries = serde_json::to_string(
      &session
        .transcript
        .iter()
        .map(SessionEntry::from)
        .collect::<Vec<_>>(),
    )
    .context("failed to serialize session transcript")?;

    let title = session.title.clone().or_else(|| session.generate_title());

    let id = if let Some(id) = session.id {
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
          session.settings.model.to_string(),
          title,
          entries,
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
           created_at, updated_at, directory, model, title, entries
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         RETURNING id",
        params![
          session.created_at,
          updated_at,
          directory,
          session.settings.model.to_string(),
          title,
          entries,
        ],
        |row| row.get(0),
      )?
    };

    self.save(&id, &snapshot)?;

    Ok(SavedSession {
      id,
      title,
      updated_at,
    })
  }

  fn snapshot(
    entries: &str,
    snapshot: Option<&str>,
  ) -> Result<SessionSnapshot> {
    match snapshot {
      Some(snapshot) => serde_json::from_str(snapshot).map_err(Into::into),
      None => SessionSnapshot::new(
        serde_json::from_str::<Vec<TranscriptEntry>>(entries)?
          .into_iter()
          .filter_map(TranscriptEntry::into_run_entry)
          .collect(),
      ),
    }
  }
}

impl SessionStore for Database {
  type Key = i64;

  fn load(&self, id: &i64) -> Result<Option<SessionSnapshot>> {
    self
      .connection
      .query_row(
        "SELECT entries, snapshot FROM sessions WHERE id = ?1",
        [id],
        |row| {
          let entries = row.get::<_, String>(0)?;
          let snapshot = row.get::<_, Option<String>>(1)?;

          Self::snapshot(&entries, snapshot.as_deref()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
              usize::from(snapshot.is_some()),
              rusqlite::types::Type::Text,
              error.into(),
            )
          })
        },
      )
      .optional()
      .with_context(|| format!("failed to load session snapshot `{id}`"))
  }

  fn save(&self, id: &i64, snapshot: &SessionSnapshot) -> Result {
    let snapshot = serde_json::to_string(snapshot)
      .context("failed to serialize session snapshot")?;

    let updated = self.connection.execute(
      "UPDATE sessions SET snapshot = ?1 WHERE id = ?2",
      params![snapshot, id],
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
        conversation: Conversation::default(),
        created_at,
        id,
        settings: Settings {
          directory: "foo".into(),
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
      conversation: Conversation::default(),
      created_at: timestamp,
      id: None,
      settings: Settings {
        directory: "foo".into(),
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
  }

  #[test]
  fn session_reads_reject_invalid_models() {
    let database =
      Database::try_from(Connection::open_in_memory().unwrap()).unwrap();

    let settings = Settings {
      directory: "foo".into(),
      max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    database
      .connection
      .execute(
        "INSERT INTO sessions (
           created_at, updated_at, directory, model, entries
         ) VALUES (0, 0, ?1, 'foo', '[]')",
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
          "INSERT INTO sessions (
             created_at, updated_at, directory, model, entries
           ) VALUES (?1, ?2, ?3, 'mock:foo', '[]')",
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
