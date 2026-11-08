use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Model {
  pub name: String,
  pub provider: String,
}

impl Model {
  pub fn new(provider: &str, name: &str) -> Result<Self> {
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
