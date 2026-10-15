use super::*;

#[test]
fn approval_prompt_approves_command() -> Result {
  #[track_caller]
  fn case(key: &str) -> Result {
    Test::new()
      .model("mock:approval-required-command")
      .submit("foo")
      .expect_screen_contains(indoc! {
        "

        ? Approve echo bar?
        y approve · n/Esc deny

        mock · approval-required-command · \
        "
      })
      .type_text("x")
      .keys([Key::Tab, Key::Up, Key::Down, Key::Enter])
      .type_text(key)
      .expect_screen_contains(indoc! {
        "

        │ foo

        baz

        ● Ran echo bar
          │ bar

        qux

        done

        │

        mock · approval-required-command · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("approval key: {key:?}"))
  }

  case("y")?;
  case("Y")
}

#[test]
fn approval_prompt_denies_command() -> Result {
  #[track_caller]
  fn case(key: &[u8]) -> Result {
    Test::new()
      .model("mock:approval-required-command")
      .submit("foo")
      .expect_screen_contains("Approve echo bar?")
      .type_text("x")
      .keys([Key::Tab, Key::Up, Key::Down, Key::Enter])
      .bytes(key)
      .expect_screen_contains(indoc! {
        "

        │ foo

        baz

        ● Failed running echo bar
          │ permission denied

        qux

        done

        │

        mock · approval-required-command · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("denial key: {}", key.escape_ascii()))
  }

  case(b"n")?;
  case(b"N")?;
  case(Key::Escape.bytes())
}

#[test]
fn approval_prompt_escapes_control_characters() -> Result {
  Test::new()
    .model("mock:escaped-command")
    .submit("foo")
    .expect_screen_contains(indoc! {
      "

      ? Approve echo foo\\r\\u{1b}[2J\\n? y approve?
      y approve · n/Esc deny

      mock · escaped-command · \
      "
    })
    .type_text("n")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Failed running echo foo\\r\\u{1b}[2J\\n? y approve
        │ permission denied

      queued for mock:escaped-command: foo

      │

      mock · escaped-command · \
      "
    })
    .quit()
    .run()
}

#[test]
fn command_output_is_limited() -> Result {
  let directory = tempfile::tempdir()?;

  fs::write(
    directory.path().join("foo"),
    format!("{}\n\nbar\nbaz\nqux\n", "foo".repeat(30)),
  )?;

  let command = if cfg!(windows) { "type foo" } else { "cat foo" };

  Test::new()
    .cwd(directory.path())
    .model("mock:command-output")
    .submit("foo")
    .expect_screen_contains(&format!("Approve {command}?"))
    .type_text("y")
    .expect_screen_contains(&format!(
      indoc! {
        "

        │ foo

        ● Ran {command}
          │ foofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoofoo...
          │ bar
          │ baz
          │ ... 1 more line

        queued for mock:command-output: foo

        │

        mock · command-output · \
        "
      },
      command = command,
    ))
    .quit()
    .run()
}

#[test]
fn command_uses_context_directory() -> Result {
  let directory = tempfile::tempdir()?;

  fs::create_dir_all(directory.path().join("bar/foo"))?;
  fs::create_dir(directory.path().join("foo"))?;

  fs::write(directory.path().join("bar/bar"), "foo")?;
  fs::write(directory.path().join("bar/foo/bar"), "baz")?;
  fs::write(directory.path().join("foo/bar"), "qux")?;

  let command = if cfg!(windows) { "type bar" } else { "cat bar" };

  for (model, output) in [
    ("command-default-directory", "foo"),
    ("command-relative-directory", "baz"),
    ("command-absolute-directory", "qux"),
  ] {
    Test::new()
      .cwd(directory.path())
      .arguments(["--directory", "bar"])
      .model(&format!("mock:{model}"))
      .submit("foo")
      .expect_screen_contains(&format!("Approve {command}?"))
      .type_text("y")
      .expect_screen_contains(&format!("● Ran {command}"))
      .expect_screen_contains(&format!(
        indoc! {
          "

            │ {output}

          queued for mock:{model}: foo

          │

          mock · {model} · \
          "
        },
        model = model,
        output = output,
      ))
      .quit()
      .run()?;
  }

  Ok(())
}

#[test]
fn ctrl_c_interrupts_active_approval() -> Result {
  Test::new()
    .model("mock:approval-required-command")
    .submit("foo")
    .expect_screen_contains("Approve echo bar?")
    .key(Key::CtrlC)
    .expect_screen_contains("Conversation interrupted")
    .type_text("bar")
    .expect_screen_contains(indoc! {
      "

      │ foo

      baz

      ● Failed running echo bar
        │ interrupted

      qux

      ■ Conversation interrupted, tell the model what to do differently.

      │ bar

      mock · approval-required-command · \
      "
    })
    .key(Key::CtrlC)
    .expect_exit(0)
    .run()
}

#[test]
fn failed_command_reports_output() -> Result {
  let directory = tempfile::tempdir()?;

  fs::create_dir(directory.path().join("foo"))?;

  Test::new()
    .cwd(directory.path())
    .model("mock:failed-command")
    .submit("foo")
    .expect_screen_contains("Approve echo bar&&echo baz>&2&&exit 1?")
    .type_text("y")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Failed running echo bar&&echo baz>&2&&exit 1
        │ cwd foo
        │ exit 1
        │ bar
        │ baz

      queued for mock:failed-command: foo

      │

      mock · failed-command · \
      "
    })
    .run()
}

#[test]
fn yolo_skips_approval() -> Result {
  Test::new()
    .model("mock:approval-required-command")
    .argument("--yolo")
    .submit("foo")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("bar")
    .expect_screen_contains("done")
    .expect_screen_excludes("Approve echo bar?")
    .run()
}
