use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Settings {
  pub(crate) directory: PathBuf,
  pub(crate) model: Model,
  pub(crate) prompt: Option<String>,
  pub(crate) yolo: bool,
}

impl Settings {
  pub(crate) fn resolve(options: Options, config: &Config) -> Result<Self> {
    let default = Model::default();

    let provider = config
      .default_provider
      .as_deref()
      .unwrap_or(&default.provider);

    let name = config.default_model.as_deref().unwrap_or(&default.name);

    let model = match options.model {
      Some(model) => model,
      None => Model::new(provider, name)?,
    };

    let directory = match options.directory {
      Some(directory) => directory,
      None => env::current_dir().context("failed to read current directory")?,
    };

    let directory = directory.canonicalize().with_context(|| {
      format!("failed to resolve directory `{}`", directory.display())
    })?;

    if !directory.is_dir() {
      bail!("`{}` is not a directory", directory.display());
    }

    Ok(Self {
      directory,
      model,
      prompt: options.prompt,
      yolo: options.yolo,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cli_model_overrides_config() {
    assert_eq!(
      Settings::resolve(
        Options {
          directory: None,
          model: Some("mock:bar".parse().unwrap()),
          prompt: None,
          yolo: false,
        },
        &Config {
          default_model: Some("foo".into()),
          default_provider: Some("mock".into()),
        },
      )
      .unwrap()
      .model,
      "mock:bar".parse().unwrap(),
    );
  }

  #[test]
  fn config_model_overrides_builtin_default() {
    assert_eq!(
      Settings::resolve(
        Options {
          directory: None,
          model: None,
          prompt: None,
          yolo: false,
        },
        &Config {
          default_model: Some("foo".into()),
          default_provider: Some("mock".into()),
        },
      )
      .unwrap()
      .model,
      "mock:foo".parse().unwrap(),
    );
  }

  #[test]
  fn directory_resolution() {
    #[track_caller]
    fn case(directory: Option<PathBuf>, expected: PathBuf) {
      assert_eq!(
        Settings::resolve(
          Options {
            directory,
            model: None,
            prompt: None,
            yolo: false,
          },
          &Config::default(),
        )
        .unwrap(),
        Settings {
          directory: expected,
          model: Model::default(),
          prompt: None,
          yolo: false,
        },
      );
    }

    let current = env::current_dir().unwrap().canonicalize().unwrap();
    let directory = tempfile::tempdir().unwrap();

    case(None, current.clone());
    case(Some(".".into()), current);
    case(
      Some(directory.path().into()),
      directory.path().canonicalize().unwrap(),
    );
  }
}
