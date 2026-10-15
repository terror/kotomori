use super::*;

#[test]
fn command_clear_interrupts_active_agent() -> Result {
  #[track_caller]
  fn case(command: &str, key: Key) -> Result {
    Test::new()
      .model("mock:slow-streaming")
      .submit("foo")
      .expect_screen_contains("queued")
      .submit("bar")
      .expect_screen_contains(indoc! {
        "

        Queued
        │ bar

        │

        mock · slow-streaming · \
        "
      })
      .type_text(command)
      .key(key)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        │

        mock · slow-streaming · \
        "
      })
      .key(Key::Up)
      .type_text("baz")
      .expect_screen_contains(indoc! {
        "

        │ baz

        mock · slow-streaming · \
        "
      })
      .key(Key::Enter)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        │ baz

        queued for mock:slow-streaming: baz

        │

        mock · slow-streaming · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("command: {command:?}, submission key: {key:?}"))
  }

  case("/clear", Key::Enter)?;
  case("/cl", Key::Enter)?;
  case("/cl", Key::AltEnter)?;
  case("  /clear  ", Key::Enter)
}

#[test]
fn command_clears() -> Result {
  #[track_caller]
  fn case(command: &str) -> Result {
    Test::new()
      .model("mock:local")
      .submit("foo")
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        │ foo

        queued for mock:local: foo

        │

        mock · local · \
        "
      })
      .submit(command)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        │

        mock · local · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("command: {command:?}"))
  }

  case("/")?;
  case("/c")?;
  case("/clear")
}

#[test]
fn command_completion_clears() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .type_text("/")
    .expect_screen_contains(indoc! {
      "

      │ /

      /clear  Clear the transcript
      /copy  Copy the last assistant response to the clipboard
      /fork  Branch the conversation to explore another approach
      /quit  Quit kotomori
      /rename  Rename the session

      mock · local · \
      "
    })
    .key(Key::Down)
    .keys([Key::Up, Key::Tab])
    .key(Key::Enter)
    .expect_screen_excludes("queued for mock:local: foo")
    .run()
}

#[test]
fn command_completion_quits() -> Result {
  #[track_caller]
  fn case(keys: &[Key]) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("/")
      .expect_screen_contains(indoc! {
        "

        │ /

        /clear  Clear the transcript
        /copy  Copy the last assistant response to the clipboard
        /fork  Branch the conversation to explore another approach
        /quit  Quit kotomori
        /rename  Rename the session

        mock · local · \
        "
      })
      .keys(keys.iter().copied())
      .key(Key::Tab)
      .expect_screen_contains(indoc! {
        "

        │ /quit

        /quit  Quit kotomori

        mock · local · \
        "
      })
      .key(Key::Enter)
      .expect_exit(0)
      .run()
      .with_context(|| format!("completion keys: {keys:?}"))
  }

  case(&[Key::Down, Key::Down, Key::Down])?;
  case(&[Key::Up, Key::Up])
}

#[test]
fn command_fork_persists_independent_sessions() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  let original = indoc! {
    "

    Type a prompt. Press Ctrl-C to quit.

    │ foo

    queued for mock:local: foo

    │

    mock · local · \
    "
  };

  let forked = indoc! {
    "

    Type a prompt. Press Ctrl-C to quit.

    │ foo

    queued for mock:local: foo

    Forked conversation.

    Session renamed to 'bar'.

    │ baz

    queued for mock:local: baz

    │

    mock · local · \
    "
  };

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains(original)
    .submit("/fork")
    .expect_screen_contains("Forked conversation.")
    .submit("/rename bar")
    .expect_screen_contains("Session renamed to 'bar'.")
    .submit("baz")
    .expect_screen_contains(forked)
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("foo")
    .expect_screen_contains(original)
    .submit("qux")
    .expect_screen_contains("queued for mock:local: qux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("bar")
    .expect_screen_contains(forked)
    .quit()
    .run()
}

#[test]
fn command_quit_interrupts_active_agent_and_saves_partial_output() -> Result {
  #[track_caller]
  fn case(command: &str, key: Key) -> Result {
    let state = tempfile::Builder::new()
      .prefix("kotomori-state")
      .tempdir()?;

    let workspace = tempfile::Builder::new()
      .prefix("kotomori-workspace")
      .tempdir()?;

    let state = state.path().to_str().unwrap();

    let interrupted = indoc! {
      "

      ■ Conversation interrupted, tell the model what to do differently.

      │

      mock · slow-streaming · \
      "
    };

    Test::new()
      .cwd(workspace.path())
      .env("KOTOMORI_HOME", state)
      .model("mock:slow-streaming")
      .submit("foo")
      .expect_screen_contains("queued")
      .type_text(command)
      .key(key)
      .expect_screen_contains(interrupted)
      .expect_screen_contains("│ foo\n\nqueued")
      .expect_screen_excludes("queued for mock:slow-streaming: foo")
      .submit("/quit")
      .expect_exit(0)
      .run()
      .with_context(|| {
        format!("command: {command:?}, submission key: {key:?}")
      })?;

    Test::new()
      .cwd(workspace.path())
      .env("KOTOMORI_HOME", state)
      .arguments(["resume", "--last"])
      .expect_screen_contains(interrupted)
      .expect_screen_contains("│ foo\n\nqueued")
      .expect_screen_excludes("queued for mock:slow-streaming: foo")
      .quit()
      .run()
      .with_context(|| format!("command: {command:?}, submission key: {key:?}"))
  }

  case("/quit", Key::Enter)?;
  case("/q", Key::Enter)?;
  case("/q", Key::AltEnter)
}

#[test]
fn command_quits() -> Result {
  #[track_caller]
  fn case(command: &str) -> Result {
    Test::new()
      .config("")
      .model("mock:local")
      .submit(command)
      .expect_exit(0)
      .run()
  }

  case("/q")?;
  case("/quit")
}

#[test]
fn command_rename_persists() -> Result {
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
    .submit("/rename")
    .expect_screen_contains("Usage: /rename <title>")
    .submit("  /rename  bar   baz  ")
    .expect_screen_contains("Session renamed to 'bar baz'.")
    .submit("qux")
    .expect_screen_contains("queued for mock:local: qux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .expect_screen_contains("> bar baz  mock:local")
    .submit("bar baz")
    .expect_screen_contains("queued for mock:local: qux")
    .submit("quux")
    .expect_screen_contains("queued for mock:local: quux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .expect_screen_contains("> bar baz  mock:local")
    .quit()
    .run()
}

#[test]
fn unknown_command() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("/foobar")
      .key(key)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        Unrecognized command '/foobar'. Type \"/\" for a list of supported commands.

        │

        mock · local · \
        "
      })
      .submit("foo")
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        Unrecognized command '/foobar'. Type \"/\" for a list of supported commands.

        │ foo

        queued for mock:local: foo

        │

        mock · local · \
        "
      })
      .run()
      .with_context(|| format!("submission key: {key:?}"))
  }

  case(Key::Enter)?;
  case(Key::AltEnter)
}
