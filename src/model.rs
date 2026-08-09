use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Model {
  pub(crate) name: String,
  pub(crate) provider: String,
}

impl Model {
  pub(crate) fn new(provider: &str, name: &str) -> Result<Self> {
    let provider = provider.trim();

    if provider.is_empty() {
      bail!("model provider cannot be empty");
    }

    let name = name.trim();

    if name.is_empty() {
      bail!("model name cannot be empty");
    }

    Ok(Self {
      name: name.into(),
      provider: provider.into(),
    })
  }
}

impl Default for Model {
  fn default() -> Self {
    Self {
      name: "local".into(),
      provider: "mock".into(),
    }
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

    Self::new(provider, name)
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
