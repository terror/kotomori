use super::*;

#[test]
fn prompt_history_edit_detaches_navigation() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .key(Key::Up)
    .type_text("!")
    .key(Key::Down)
    .type_text("?")
    .expect_screen_contains(indoc! {
      "

      │ foo!?

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::Down)
    .expect_screen_contains(indoc! {
      "

      │ foo!?

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_is_cleared_by_clear_command() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .submit("/clear")
    .key(Key::Up)
    .type_text("bar")
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_loads_session() -> Result {
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
    .type_text("baz")
    .key(Key::CtrlJ)
    .submit("qux")
    .expect_screen_contains("queued for mock:local: baz qux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .arguments(["resume", "--last"])
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ baz
      │ qux

      mock · local · \
      "
    })
    .keys([Key::Up, Key::Up])
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_navigates_and_restores_draft() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .submit("bar")
    .expect_screen_contains("queued for mock:local: bar")
    .type_text("baz")
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::Down)
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · local · \
      "
    })
    .key(Key::Down)
    .expect_screen_contains(indoc! {
      "

      │ baz

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_preserves_multiline_navigation() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .type_text("bar")
    .key(Key::CtrlJ)
    .type_text("baz")
    .key(Key::Up)
    .type_text("!")
    .expect_screen_contains(indoc! {
      "

      │ bar!
      │ baz

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::Down)
    .type_text("?")
    .expect_screen_contains(indoc! {
      "

      │ bar!
      │ baz?

      mock · local · \
      "
    })
    .quit()
    .run()
}
