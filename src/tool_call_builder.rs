use super::*;

#[derive(Default)]
pub(crate) struct ToolCallBuilder {
  arguments: ToolCallArguments,
  id: Option<String>,
  name: Option<String>,
}

impl ToolCallBuilder {
  pub(crate) fn argument_fragment(
    self,
    argument_fragment: Option<&str>,
  ) -> Self {
    Self {
      arguments: self.arguments.argument_fragment(argument_fragment),
      ..self
    }
  }

  pub(crate) fn arguments(self, arguments: Option<Value>) -> Self {
    Self {
      arguments: self.arguments.arguments(arguments),
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

  pub(crate) fn id(self, id: Option<String>) -> Self {
    Self {
      id: id.or(self.id),
      ..self
    }
  }

  pub(crate) fn name(self, name: Option<String>) -> Self {
    Self {
      name: name.or(self.name),
      ..self
    }
  }
}
