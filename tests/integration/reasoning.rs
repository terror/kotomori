use super::*;

#[test]
fn ctrl_t_toggles_reasoning() -> Result {
  let collapsed = indoc! {
    "

    │ foo

    corge

    Thinking...

    grault

    │

    mock · reasoning · \
    "
  };

  Test::new()
    .model("mock:reasoning")
    .submit("foo")
    .expect_screen_contains(collapsed)
    .key(Key::CtrlT)
    .expect_screen_contains(indoc! {
      "

      │ foo

      corge

      Thinking...
        │ bar
        │ baz
        │ qux

      grault

      │

      mock · reasoning · \
      "
    })
    .key(Key::CtrlT)
    .expect_screen_contains(collapsed)
    .run()
}

#[test]
fn empty_reasoning_is_hidden() -> Result {
  #[track_caller]
  fn case(model: &str, expanded: bool) -> Result {
    let test = Test::new().model(&format!("mock:{model}"));

    let test = if expanded { test.key(Key::CtrlT) } else { test };

    test
      .submit("foo")
      .expect_screen_contains(&format!(
        indoc! {
          "

          Type a prompt. Press Ctrl-C to quit.

          │ foo

          │

          mock · {model} · \
          "
        },
        model = model,
      ))
      .run()
      .with_context(|| format!("model: {model:?}, expanded: {expanded}"))
  }

  for model in ["empty-reasoning", "encrypted-reasoning"] {
    case(model, false)?;
    case(model, true)?;
  }

  Ok(())
}

#[test]
fn reasoning_survives_interruption() -> Result {
  Test::new()
    .model("mock:unfinished-reasoning")
    .submit("foo")
    .expect_screen_contains(indoc! {
      "

      │ foo

      bar

      Thinking...

      quux

      Thinking...

      corge grault

      │

      mock · unfinished-reasoning · \
      "
    })
    .key(Key::CtrlT)
    .expect_screen_contains(indoc! {
      "

      │ foo

      bar

      Thinking...
        │ baz
        │ qux

      quux

      Thinking...
        │ quuz

      corge grault

      │

      mock · unfinished-reasoning · \
      "
    })
    .key(Key::CtrlC)
    .expect_screen_contains(indoc! {
      "

      │ foo

      bar

      Thinking...
        │ baz
        │ qux

      quux

      Thinking...
        │ quuz

      corge grault

      ■ Conversation interrupted, tell the model what to do differently.

      │

      mock · unfinished-reasoning · \
      "
    })
    .key(Key::CtrlT)
    .expect_screen_contains(indoc! {
      "

      │ foo

      bar

      Thinking...

      quux

      Thinking...

      corge grault

      ■ Conversation interrupted, tell the model what to do differently.

      │

      mock · unfinished-reasoning · \
      "
    })
    .quit()
    .run()
}
