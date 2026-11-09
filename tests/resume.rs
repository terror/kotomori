use super::*;

#[test]
fn resume_and_fork_preserve_unknown_extension_entries() -> Result {
  let directory = tempfile::tempdir()?;
  let state = tempfile::tempdir()?;
  let home = state.path().to_str().unwrap();

  Test::new()
    .cwd(directory.path())
    .env("KOTOMORI_HOME", home)
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .quit()
    .run()?;

  let connection = Connection::open(state.path().join("kotomori.db"))?;
  let snapshot = connection.query_row(
    "SELECT snapshot FROM sessions WHERE id = 1",
    [],
    |row| row.get::<_, String>(0),
  )?;
  let mut entries =
    serde_json::from_str::<SessionSnapshot>(&snapshot)?.into_entries();
  let custom = SessionEntry::Custom(CustomEntry {
    data: serde_json::json!({"foo": [true, "corge", null]}),
    kind: "bar".into(),
    namespace: "baz".into(),
    version: 42,
  });
  entries.insert(1, custom.clone());
  connection.execute(
    "UPDATE sessions SET snapshot = ?1 WHERE id = 1",
    [serde_json::to_string(&SessionSnapshot::new(entries)?)?],
  )?;

  Test::new()
    .cwd(directory.path())
    .env("KOTOMORI_HOME", home)
    .arguments(["resume", "--last"])
    .submit("/rename bar")
    .expect_screen_contains("Session renamed to 'bar'.")
    .submit("baz")
    .expect_screen_contains("queued for mock:local: baz")
    .submit("/fork")
    .expect_screen_contains("Forked conversation.")
    .submit("/rename qux")
    .expect_screen_contains("Session renamed to 'qux'.")
    .submit("quux")
    .expect_screen_contains("queued for mock:local: quux")
    .expect_screen_excludes("corge")
    .quit()
    .run()?;

  for (title, response) in [("bar", "baz"), ("qux", "quux")] {
    Test::new()
      .cwd(directory.path())
      .env("KOTOMORI_HOME", home)
      .argument("resume")
      .submit(title)
      .expect_screen_contains(&format!("queued for mock:local: {response}"))
      .expect_screen_excludes("corge")
      .quit()
      .run()?;
  }

  let mut statement =
    connection.prepare("SELECT snapshot FROM sessions ORDER BY id")?;
  let snapshots = statement
    .query_map([], |row| row.get::<_, String>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  assert_eq!(snapshots.len(), 2);

  for snapshot in snapshots {
    let snapshot = serde_json::from_str::<SessionSnapshot>(&snapshot)?;
    assert_eq!(
      snapshot
        .entries()
        .iter()
        .filter(|entry| matches!(entry, SessionEntry::Custom(entry) if entry.namespace == "baz"))
        .collect::<Vec<_>>(),
      [&custom],
    );
  }

  Ok(())
}

#[test]
fn resume_filters_and_loads_session() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let other_workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(other_workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("qux")
    .expect_screen_contains("queued for mock:local: qux")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .env("KOTOMORI_HOME", state)
    .arguments(["--directory", workspace.path().to_str().unwrap()])
    .model("mock:local")
    .submit("bar")
    .expect_screen_contains("queued for mock:local: bar")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .arguments(["--directory", "."])
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .env("KOTOMORI_HOME", state)
    .model("mock:other")
    .arguments(["resume", "--directory", workspace.path().to_str().unwrap()])
    .expect_screen_contains("foo  mock:local")
    .expect_screen_contains("bar  mock:local")
    .expect_screen_excludes("qux  mock:local")
    .type_text("bar")
    .expect_screen_excludes("foo  mock:local")
    .key(Key::Enter)
    .expect_screen_contains("queued for mock:local: bar")
    .submit("baz")
    .expect_screen_contains("queued for mock:local: baz")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .arguments(["resume", "--last"])
    .submit("latest")
    .expect_screen_contains("queued for mock:local: latest")
    .run()
}

#[test]
fn resume_interrupts_pending_tool_calls() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:approval-required-command")
    .submit("foo")
    .expect_screen_contains("Approve echo bar?")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .arguments(["resume", "--last"])
    .expect_screen_contains(indoc! {
      "

      │ foo

      baz

      ● Interrupted running echo bar
        │ interrupted

      qux

      │

      mock · approval-required-command · \
      "
    })
    .submit("bar")
    .expect_screen_contains(indoc! {
      "

      │ bar

      done

      │

      mock · approval-required-command · \
      "
    })
    .quit()
    .run()
}

#[test]
fn resume_lists_sessions_with_invalid_snapshots() -> Result {
  let directory = tempfile::tempdir()?;

  let state = tempfile::tempdir()?;

  Test::new()
    .cwd(directory.path())
    .env("KOTOMORI_HOME", state.path().to_str().unwrap())
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .quit()
    .run()?;

  Connection::open(state.path().join("kotomori.db"))?
    .execute("UPDATE sessions SET snapshot = '{}', title = NULL", [])?;

  Test::new()
    .cwd(directory.path())
    .env("KOTOMORI_HOME", state.path().to_str().unwrap())
    .argument("resume")
    .expect_screen_contains("> Untitled session  mock:local")
    .key(Key::Escape)
    .expect_exit(0)
    .run()?;

  Test::new()
    .cwd(directory.path())
    .env("KOTOMORI_HOME", state.path().to_str().unwrap())
    .arguments(["resume", "--last"])
    .stderr(indoc! {
      "
      error: failed to load session `1`

      because:
      - Conversion error from type Text at index: 6, missing field `entries` at line 1 column 2
      - missing field `entries` at line 1 column 2
      "
    })
    .status(1)
}

#[test]
fn resume_loads_sessions_with_tools_and_interruptions() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:approval-required-command")
    .submit("approved tool result")
    .expect_screen_contains("Approve echo bar?")
    .type_text("y")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("done")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:approval-required-command")
    .submit("denied tool result")
    .expect_screen_contains("Approve echo bar?")
    .type_text("n")
    .expect_screen_contains("Denied running echo bar")
    .expect_screen_contains("permission denied")
    .expect_screen_contains("done")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:slow-streaming")
    .submit("interrupted response")
    .expect_screen_contains("queued")
    .key(Key::CtrlC)
    .expect_screen_contains("Conversation interrupted")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("approved tool result")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("done")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("denied tool result")
    .expect_screen_contains("Denied running echo bar")
    .expect_screen_contains("permission denied")
    .expect_screen_contains("done")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("interrupted response")
    .expect_screen_contains("queued")
    .expect_screen_contains("Conversation interrupted")
    .run()
}

#[test]
fn resume_picker_cancels_with_escape_and_ctrl_c() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .key(Key::Escape)
    .expect_exit(0)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .key(Key::CtrlC)
    .expect_exit(0)
    .run()
}

#[test]
fn resume_with_no_sessions_exits_successfully() -> Result {
  let directory = tempfile::tempdir()?;

  Test::new()
    .env("KOTOMORI_HOME", directory.path().to_str().unwrap())
    .argument("resume")
    .stdout("No saved sessions.\n")
    .success()?;

  assert_eq!(
    Connection::open(directory.path().join("kotomori.db"))?.query_row(
      "PRAGMA user_version",
      [],
      |row| row.get::<_, i64>(0)
    )?,
    2,
  );

  Ok(())
}
