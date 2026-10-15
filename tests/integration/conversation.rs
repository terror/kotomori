use super::*;

#[test]
fn ctrl_c_quits_gracefully() -> Result {
  Test::new()
    .expect_screen_contains(indoc! {
      "

      Type a prompt. Press Ctrl-C to quit.

      │

      mock · local · \
      "
    })
    .key(Key::CtrlC)
    .expect_exit(0)
    .run()
}

#[test]
fn immediate_submit_interrupts_active_agent_and_starts_new_run() -> Result {
  let queued = indoc! {
    "

    Queued
    │ baz

    │

    mock · slow-streaming · \
    "
  };

  Test::new()
    .model("mock:slow-streaming")
    .submit("foo")
    .expect_screen_contains("queued")
    .submit("baz")
    .expect_screen_contains(queued)
    .type_text("  bar  ")
    .key(Key::AltEnter)
    .expect_screen_contains("Conversation interrupted")
    .expect_screen_contains(queued)
    .expect_screen_contains(indoc! {
      "

      ■ Conversation interrupted, tell the model what to do differently.

      │   bar

      queued for mock:slow-streaming:   bar

      │ baz

      queued for mock:slow-streaming: baz

      │

      mock · slow-streaming · \
      "
    })
    .expect_screen_contains("│ foo\n\nqueued")
    .expect_screen_excludes("queued for mock:slow-streaming: foo")
    .key(Key::Up)
    .type_text("!")
    .expect_screen_contains(indoc! {
      "

      │   bar  !

      mock · slow-streaming · \
      "
    })
    .run()
}

#[test]
fn interrupt_active_agent() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    let test = Test::new()
      .model("mock:slow-streaming")
      .submit("foo")
      .expect_screen_contains("queued")
      .key(key)
      .expect_screen_contains("Conversation interrupted");

    let test = if matches!(key, Key::Escape) {
      test
        .key(Key::Escape)
        .wait(SETTLE_INTERVAL)
        .key(Key::Up)
        .expect_screen_contains(indoc! {
          "

          │ foo

          mock · slow-streaming · \
          "
        })
    } else {
      test
    };

    test
      .key(Key::CtrlC)
      .expect_exit(0)
      .run()
      .with_context(|| format!("interruption key: {key:?}"))
  }

  case(Key::CtrlC)?;
  case(Key::Escape)
}

#[test]
fn interrupt_advances_to_next_queued_submission() -> Result {
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
    .key(Key::Escape)
    .expect_screen_contains(indoc! {
      "

      ■ Conversation interrupted, tell the model what to do differently.

      │ bar

      queued for mock:slow-streaming: bar

      │

      mock · slow-streaming · \
      "
    })
    .expect_screen_excludes("queued for mock:slow-streaming: foo")
    .run()
}

#[test]
fn markdown_response() -> Result {
  Test::new()
    .model("mock:slow-streaming")
    .submit("**foo**")
    .expect_screen_contains("│ **foo**")
    .expect_screen_contains("queued for mock:slow-streaming: foo")
    .run()
}

#[test]
fn prompt_round_trip() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .run()
}

#[test]
fn provider_error_recovers() -> Result {
  Test::new()
    .model("mock:error")
    .submit("foo")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Error
        │ foo
        │ bar

      │

      mock · error · \
      "
    })
    .submit("bar")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Error
        │ foo
        │ bar

      │ bar

      queued for mock:error: bar

      │

      mock · error · \
      "
    })
    .run()
}

#[test]
fn provider_malformed_tool_arguments_recovers() -> Result {
  Test::new()
    .model("mock:malformed-tool-arguments")
    .submit("foo")
    .expect_screen_contains("failed to decode `command` arguments")
    .submit("bar")
    .expect_screen_contains("queued for mock:malformed-tool-arguments: bar")
    .run()
}

#[test]
fn provider_unknown_tool_recovers() -> Result {
  Test::new()
    .model("mock:unknown-tool")
    .submit("foo")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Error
        │ unknown tool `unknown`

      │

      mock · unknown-tool · \
      "
    })
    .submit("bar")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Error
        │ unknown tool `unknown`

      │ bar

      queued for mock:unknown-tool: bar

      │

      mock · unknown-tool · \
      "
    })
    .run()
}

#[test]
fn queued_submissions_run_in_order() -> Result {
  Test::new()
    .model("mock:slow-streaming")
    .submit("foo")
    .expect_screen_contains("queued")
    .type_text("bar")
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · slow-streaming · \
      "
    })
    .key(Key::Enter)
    .type_text("baz")
    .keys([Key::CtrlJ, Key::CtrlJ])
    .type_text("qux")
    .keys([Key::CtrlJ, Key::Enter])
    .expect_screen_contains(indoc! {
      "

      Queued
      │ bar

      Queued
      │ baz
      │
      │ qux
      │

      │

      mock · slow-streaming · \
      "
    })
    .expect_screen_contains(indoc! {
      "

      │ foo

      queued for mock:slow-streaming: foo

      │ bar

      queued for mock:slow-streaming: bar

      │ baz
      │
      │ qux
      │

      queued for mock:slow-streaming: baz

      qux

      │

      mock · slow-streaming · \
      "
    })
    .run()
}

#[test]
fn second_turn_conversation() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .submit("bar")
    .expect_screen_contains("queued for mock:local: bar")
    .run()
}
