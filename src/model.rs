use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Model {
  name: String,
  provider: ProviderName,
}

impl Model {
  pub(crate) fn name(&self) -> &str {
    &self.name
  }

  pub(crate) fn provider(&self) -> ProviderName {
    self.provider
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

    if name.is_empty() {
      bail!("model name cannot be empty");
    }

    Ok(Self {
      name: name.into(),
      provider: provider.parse()?,
    })
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderName {
  Fake,
  Ollama,
}

impl Display for ProviderName {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Fake => write!(f, "fake"),
      Self::Ollama => write!(f, "ollama"),
    }
  }
}

impl FromStr for ProviderName {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self> {
    match s {
      "fake" => Ok(Self::Fake),
      "ollama" => Ok(Self::Ollama),
      provider => bail!("unknown provider `{provider}`"),
    }
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
        provider: ProviderName::Fake,
      },
    );

    case(
      "ollama:bar",
      &Model {
        name: "bar".into(),
        provider: ProviderName::Ollama,
      },
    );
  }
}
