use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawToolCall {
  pub(crate) arguments: Value,
  pub(crate) id: String,
  pub(crate) name: String,
}

impl RawToolCall {
  pub(crate) fn from_arguments_string(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl AsRef<str>,
  ) -> Result<Self> {
    let name = name.into();

    let arguments = serde_json::from_str(arguments.as_ref())
      .with_context(|| format!("failed to parse `{name}` arguments"))?;

    Ok(Self::new(id, name, arguments))
  }

  pub(crate) fn into_invocation(self) -> Result<ToolInvocation> {
    let name = self.name.clone();

    inventory::iter::<RegisteredTool>
      .into_iter()
      .find(|tool| tool.name == name)
      .with_context(|| format!("unknown tool `{name}`"))?
      .invocation(self)
  }

  pub(crate) fn invocation(&self) -> Result<ToolInvocation> {
    self.clone().into_invocation()
  }

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
