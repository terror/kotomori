use super::*;

#[test]
fn config_sets_default_model() -> Result {
  Test::new()
    .config(
      r#"
      default_provider = "mock"
      default_model = "bar"
      "#,
    )
    .submit("foo")
    .expect_screen_contains("queued for mock:bar: foo")
    .run()
}

#[test]
fn dev_mode_controls_time_to_first_draw() -> Result {
  Test::new()
    .model("mock:local")
    .wait(SETTLE_INTERVAL)
    .expect_screen_excludes("first draw ")
    .run()?;

  Test::new()
    .env("KOTOMORI_DEV", "1")
    .model("mock:local")
    .wait(SETTLE_INTERVAL)
    .expect_screen_contains("first draw ")
    .run()
}

#[test]
fn directory_is_file() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("foo");

  fs::write(&path, "bar")?;

  Test::new()
    .arguments(["--directory", path.to_str().unwrap()])
    .stderr(&format!(
      "error: `{}` is not a directory\n",
      path.canonicalize()?.display()
    ))
    .status(1)
}

#[test]
fn directory_is_missing() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("foo");

  Test::new()
    .arguments(["--directory", path.to_str().unwrap()])
    .stderr(&format!(
      "error: failed to resolve directory `{}`\n\nbecause:\n- {}\n",
      path.display(),
      io::Error::from_raw_os_error(2),
    ))
    .status(1)
}

#[test]
fn initial_prompt_submits() -> Result {
  Test::new()
    .model("mock:local")
    .arguments(["--prompt", "foo"])
    .key(Key::Enter)
    .expect_screen_contains("queued for mock:local: foo")
    .run()
}

#[test]
fn missing_config_defaults() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("foo");

  Test::new()
    .env("KOTOMORI_CONFIG", path.to_str().unwrap())
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .quit()
    .run()?;

  assert!(path.is_file());

  assert_eq!(
    confy::load_path::<BTreeMap<String, String>>(&path)?,
    BTreeMap::new(),
  );

  Ok(())
}

#[test]
fn model_argument_errors() -> Result {
  fn case(model: &str, error: &str) -> Result {
    Test::new()
      .model(model)
      .stderr(&format!(
        indoc! {
          "
          error: invalid value '{model}' for '--model <MODEL>': {error}

          For more information, try '--help'.
          "
        },
        model = model,
        error = error,
      ))
      .status(2)
      .with_context(|| format!("model: {model:?}"))
  }

  case("foo: ", "model name cannot be empty")?;
  case(":foo", "model provider cannot be empty")?;
  case("foo", "model must be PROVIDER:MODEL")
}

#[test]
fn model_argument_parses() -> Result {
  fn case(model: &str, expected: &str) -> Result {
    Test::new()
      .model(model)
      .submit("foo")
      .expect_screen_contains(&format!(
        indoc! {
          "

          │ foo

          queued for mock:{model}: foo

          │

          mock · {model} · \
          "
        },
        model = expected,
      ))
      .run()
      .with_context(|| format!("model: {model:?}"))
  }

  case("mock:bar", "bar")?;
  case("mock:bar:baz", "bar:baz")?;
  case("mock: bar ", "bar")
}

#[test]
fn unknown_provider() -> Result {
  fn case(test: Test) -> Result {
    test
      .stderr("error: no registered provider is named `foo`\n")
      .status(1)
  }

  case(Test::new().model("foo:bar"))?;
  case(Test::new().config(
    r#"
    default_provider = "foo"
    default_model = "bar"
    "#,
  ))
}

#[test]
fn unsupported_database_schema() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("kotomori.db");

  Connection::open(&path)?.execute_batch("PRAGMA user_version = 2")?;

  Test::new()
    .env("KOTOMORI_HOME", directory.path().to_str().unwrap())
    .stderr(&format!(
      indoc! {
        "
        error: failed to open database `{path}`

        because:
        - database schema version 2 is unsupported; expected 1
        "
      },
      path = path.display(),
    ))
    .status(1)
}
