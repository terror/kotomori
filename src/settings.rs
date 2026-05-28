use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Settings {
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

    Ok(Self {
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
  fn missing_config_uses_builtin_default() {
    assert_eq!(
      Settings::resolve(
        Options {
          model: None,
          prompt: None,
          yolo: false,
        },
        &Config::default(),
      )
      .unwrap()
      .model,
      Model::default(),
    );
  }
}
