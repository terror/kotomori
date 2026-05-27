use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Model {
  name: String,
  provider: String,
}

impl Model {
  pub(crate) fn name(&self) -> &str {
    &self.name
  }

  pub(crate) fn provider(&self) -> &str {
    &self.provider
  }
}

impl Display for Model {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}:{}", self.provider, self.name)
  }
}

impl FromStr for Model {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self> {
    let Some((provider, name)) = s.split_once(':') else {
      bail!("model must be PROVIDER:MODEL");
    };

    let name = name.trim();

    if provider.is_empty() {
      bail!("model provider cannot be empty");
    }

    if name.is_empty() {
      bail!("model name cannot be empty");
    }

    Ok(Self {
      name: name.into(),
      provider: provider.into(),
    })
  }
}

impl TryFrom<Model> for Arc<dyn Provider> {
  type Error = Error;

  fn try_from(model: Model) -> Result<Self> {
    match model.provider.as_str() {
      "anthropic" => Ok(Arc::new(Rig::anthropic(&model)?)),
      "deepseek" => Ok(Arc::new(Rig::deepseek(&model)?)),
      "fake" => Ok(Arc::new(Fake)),
      "groq" => Ok(Arc::new(Rig::groq(&model)?)),
      "ollama" => Ok(Arc::new(Rig::ollama(&model)?)),
      "openai" => Ok(Arc::new(Rig::openai(&model)?)),
      "openrouter" => Ok(Arc::new(Rig::openrouter(&model)?)),
      provider => bail!("unknown provider `{provider}`"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn errors_on_empty_name() {
    assert_eq!(
      "foo: ".parse::<Model>().unwrap_err().to_string(),
      "model name cannot be empty",
    );
  }

  #[test]
  fn errors_on_empty_provider() {
    assert_eq!(
      ":foo".parse::<Model>().unwrap_err().to_string(),
      "model provider cannot be empty",
    );
  }

  #[test]
  fn errors_on_missing_separator() {
    assert_eq!(
      "foo".parse::<Model>().unwrap_err().to_string(),
      "model must be PROVIDER:MODEL",
    );
  }

  #[test]
  fn parses_provider_and_name() {
    assert_eq!(
      "foo:bar".parse::<Model>().unwrap(),
      Model {
        name: "bar".into(),
        provider: "foo".into(),
      },
    );
  }

  #[test]
  fn preserves_colons_in_name() {
    assert_eq!(
      "foo:bar:baz".parse::<Model>().unwrap(),
      Model {
        name: "bar:baz".into(),
        provider: "foo".into(),
      },
    );
  }

  #[test]
  fn trims_name() {
    assert_eq!(
      "foo: bar ".parse::<Model>().unwrap(),
      Model {
        name: "bar".into(),
        provider: "foo".into(),
      },
    );
  }
}
