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

impl FromSql for Model {
  fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
    value
      .as_str()?
      .parse::<Self>()
      .map_err(|error| FromSqlError::Other(error.into()))
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
