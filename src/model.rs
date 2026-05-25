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

impl From<Model> for Arc<dyn Provider> {
  fn from(model: Model) -> Self {
    match model.provider.as_str() {
      "fake" => Arc::new(Fake),
      "ollama" => Arc::new(Ollama::new()),
      provider => panic!("unknown provider `{provider}`"),
    }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parsing() {
    #[track_caller]
    fn case(s: &str, expected: &Model) {
      assert_eq!(&s.parse::<Model>().unwrap(), expected);
    }

    case(
      "fake:foo",
      &Model {
        name: "foo".into(),
        provider: "fake".into(),
      },
    );

    case(
      "ollama:bar",
      &Model {
        name: "bar".into(),
        provider: "ollama".into(),
      },
    );

    case(
      "other:baz",
      &Model {
        name: "baz".into(),
        provider: "other".into(),
      },
    );
  }
}
