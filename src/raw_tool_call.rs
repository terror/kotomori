use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawToolCall {
  pub(crate) arguments: Value,
  pub(crate) id: String,
  pub(crate) name: String,
}

impl RawToolCall {
  pub(crate) fn new(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: Value,
  ) -> Self {
    Self {
      arguments,
      id: id.into(),
      name: name.into(),
    }
  }
}

impl Display for RawToolCall {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{} {}", self.name, self.arguments)
  }
}

impl TryInto<ToolInvocation> for RawToolCall {
  type Error = Error;

  fn try_into(self) -> Result<ToolInvocation> {
    let name = self.name.clone();
    let id = self.id.clone();

    let kind = tools::TOOLS
      .iter()
      .find(|tool| tool.name == name)
      .with_context(|| format!("unknown tool `{name}`"))?
      .invocation(self)?;

    Ok(ToolInvocation { id, kind })
  }
}
