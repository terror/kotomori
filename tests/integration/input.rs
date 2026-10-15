use super::*;

#[test]
fn blank_submit_does_nothing() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("  ")
      .key(key)
      .type_text("foo")
      .expect_screen_contains(indoc! {
        "

        │   foo

        mock · local · \
        "
      })
      .key(Key::CtrlC)
      .expect_exit(0)
      .expect_screen_excludes("queued for mock:local:")
      .run()
      .with_context(|| format!("submission key: {key:?}"))
  }

  case(Key::Enter)?;
  case(Key::AltEnter)
}

#[cfg_attr(not(unix), ignore = "crossterm only emits paste events on Unix")]
#[test]
fn bracketed_paste_inserts_one_edit() -> Result {
  let input = indoc! {
    "

    │ foobar
    │ baz
    │ qux
    │ quux

    mock · local · \
    "
  };

  Test::new()
    .model("mock:local")
    .type_text("foo")
    .paste("bar\rbaz\r\nqux\nquux")
    .expect_screen_contains(input)
    .expect_screen_excludes("queued for mock:local:")
    .key(Key::CtrlU)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::CtrlR)
    .expect_screen_contains(input)
    .key(Key::Enter)
    .expect_screen_contains(indoc! {
      "

      │ foobar
      │ baz
      │ qux
      │ quux

      queued for mock:local: foobar baz qux quux

      │

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn multiline_input() -> Result {
  let input = indoc! {
    "

    │ foofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofo
    │ o
    │ bar

    mock · local · \
    "
  };

  Test::new()
    .model("mock:local")
    .type_text(&"foo".repeat(25))
    .key(Key::CtrlJ)
    .type_text("bar")
    .expect_screen_contains(input)
    .key(Key::Enter)
    .expect_screen_contains(indoc! {
      "

      │ foofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofo
      │ o
      │ bar

      queued for mock:local: foofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofo
      ofoofoofoofoofoofoofoo bar

      │

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(input)
    .run()
}

#[test]
fn submit_preserves_input_whitespace() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("  foo")
      .key(Key::CtrlJ)
      .type_text("  ")
      .key(key)
      .expect_screen_contains(indoc! {
        "

        │   foo
        │

        queued for mock:local:   foo

        │

        mock · local · \
        "
      })
      .key(Key::Up)
      .type_text("!")
      .expect_screen_contains(indoc! {
        "

        │   foo
        │   !

        mock · local · \
        "
      })
      .run()
      .with_context(|| format!("submission key: {key:?}"))
  }

  case(Key::Enter)?;
  case(Key::AltEnter)
}
