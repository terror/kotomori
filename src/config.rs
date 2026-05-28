use super::*;

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
  pub(crate) default_model: Option<String>,
  pub(crate) default_provider: Option<String>,
}

impl Config {
  const APP_NAME: &'static str = "kotomori";
  const CONFIG_NAME: &'static str = "config";

  pub(crate) fn load() -> Result<Self> {
    if let Some(path) = env::var_os("KOTOMORI_CONFIG") {
      confy::load_path(PathBuf::from(path))
        .context("failed to load configuration")
    } else {
      confy::load(Self::APP_NAME, Some(Self::CONFIG_NAME))
        .context("failed to load configuration")
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn missing_config_defaults() {
    let directory = tempfile::tempdir().unwrap();

    let path = directory.path().join("config.toml");

    assert_eq!(
      confy::load_path::<Config>(&path).unwrap(),
      Config::default(),
    );

    assert!(path.is_file());
  }

  #[test]
  fn parses_default_model() {
    let directory = tempfile::tempdir().unwrap();

    let path = directory.path().join("config.toml");

    fs::write(
      &path,
      r#"
      default_provider = "mock"
      default_model = "foo"
      "#,
    )
    .unwrap();

    assert_eq!(
      confy::load_path::<Config>(&path).unwrap(),
      Config {
        default_model: Some("foo".into()),
        default_provider: Some("mock".into()),
      },
    );
  }
}
