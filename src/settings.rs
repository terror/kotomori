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
  fn config_overrides() {
    #[track_caller]
    fn case(model: Option<&str>, expected: &str) {
      assert_eq!(
        Settings::resolve(
          Options {
            directory: None,
            model: model.map(|model| model.parse().unwrap()),
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
        expected.parse().unwrap(),
      );
    }

    case(None, "mock:foo");
    case(Some("mock:bar"), "mock:bar");
  }
}
