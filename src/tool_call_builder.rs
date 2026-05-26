use super::*;

#[derive(Default)]
pub(crate) struct ToolCallBuilder {
  arguments: ToolCallArguments,
  id: Option<String>,
  name: Option<String>,
}

impl ToolCallBuilder {
  pub(crate) fn argument_delta(self, argument_delta: &str) -> Result<Self> {
    Ok(Self {
      arguments: self.arguments.argument_delta(argument_delta)?,
      ..self
    })
  }

  pub(crate) fn arguments(self, arguments: Value) -> Self {
    Self {
      arguments: ToolCallArguments::Value(arguments),
      ..self
    }
  }

  pub(crate) fn finish(self) -> Result<RawToolCall> {
    Ok(RawToolCall::new(
      self.id.context("missing tool call id")?,
      self.name.context("missing tool call name")?,
      self.arguments.finish()?,
    ))
  }

  pub(crate) fn id(self, id: String) -> Self {
    Self {
      id: Some(id),
      ..self
    }
  }

  pub(crate) fn name(self, name: String) -> Self {
    Self {
      name: Some(name),
      ..self
    }
  }
}
