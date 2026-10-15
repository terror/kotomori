use super::*;

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
    .expect_screen_contains("foo")
    .expect_screen_contains("bar")
    .expect_screen_excludes("qux")
    .type_text("bar")
    .expect_screen_excludes("foo")
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

      ● Failed running echo bar
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
fn resume_lists_sessions_with_invalid_transcripts() -> Result {
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
    .execute("UPDATE sessions SET entries = '{}', title = NULL", [])?;

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
      - Conversion error from type Text at index: 6, invalid type: map, expected a sequence at line 1 column 0
      - invalid type: map, expected a sequence at line 1 column 0
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
    .expect_screen_contains("Failed running echo bar")
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
    .expect_screen_contains("Failed running echo bar")
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
fn resume_matches_queries() -> Result {
  let directory = tempfile::tempdir()?;

  let workspace = directory.path().join("foo");

  fs::create_dir(&workspace)?;

  let state = tempfile::tempdir()?;

  Test::new()
    .cwd(&workspace)
    .env("KOTOMORI_HOME", state.path().to_str().unwrap())
    .model("mock:bar")
    .submit("bazbazbaz")
    .expect_screen_contains("queued for mock:bar: bazbazbaz")
    .quit()
    .run()?;

  for (title, queries) in [
    (
      Some("bazbazbaz"),
      [
        ("", true),
        ("  BAZBAZBAZ  MOCK:BAR FOO 1234567890123456789  ", true),
        ("bazbazbaz 1234567890123456788", false),
        ("quxquxqux", false),
        ("Untitled", false),
      ]
      .as_slice(),
    ),
    (
      None,
      [
        ("UNTITLED SESSION 1234567890123456789", true),
        ("bazbazbaz", false),
      ]
      .as_slice(),
    ),
  ] {
    Connection::open(state.path().join("kotomori.db"))?.execute(
      "UPDATE sessions SET id = 1234567890123456789, title = ?1",
      [title],
    )?;

    let test = Test::new()
      .cwd(&workspace)
      .env("KOTOMORI_HOME", state.path().to_str().unwrap())
      .argument("resume");

    let test = queries.iter().fold(test, |test, (query, matches)| {
      let search = format!("Search: {query}");

      let expected = if *matches {
        format!("> {}  mock:bar", title.unwrap_or("Untitled session"))
      } else {
        "No matching sessions.".into()
      };

      test
        .key(Key::CtrlU)
        .paste(query)
        .expect_screen_contains(&format!("{}\n\n{expected}", search.trim_end()))
    });

    test.key(Key::Escape).expect_exit(0).run()?;
  }

  Ok(())
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
    1,
  );

  Ok(())
}
