use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct ToolName(String);

impl ToolName {
  #[must_use]
  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn new(name: impl Into<String>) -> Result<Self> {
    let name = name.into();

    if name.is_empty() {
      bail!("a tool name cannot be empty");
    }

    Ok(Self(name))
  }
}

impl Display for ToolName {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    Display::fmt(&self.0, f)
  }
}

impl From<ToolName> for String {
  fn from(name: ToolName) -> Self {
    name.0
  }
}

impl TryFrom<String> for ToolName {
  type Error = Error;

  fn try_from(name: String) -> Result<Self> {
    Self::new(name)
  }
}
