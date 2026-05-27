use super::*;

/// Builds a complete tool call from independently streamed fields.
///
/// Provider protocols may deliver the id, name, and argument JSON in separate
/// events. The builder holds those partial fields until the stream reports that
/// the call is finished, at which point missing required fields become errors.
#[derive(Default)]
pub(crate) struct ToolCallBuilder {
  arguments: ToolCallArguments,
  id: Option<String>,
  name: Option<String>,
}

impl ToolCallBuilder {
  pub(crate) fn argument_delta(&mut self, argument_delta: &str) -> &mut Self {
    self.arguments.argument_delta(argument_delta);
    self
  }

  pub(crate) fn finish(self) -> Result<RawToolCall> {
    Ok(RawToolCall::new(
      self.id.context("missing tool call id")?,
      self.name.context("missing tool call name")?,
      self.arguments.finish()?,
    ))
  }

  pub(crate) fn id(&mut self, id: String) -> &mut Self {
    self.id = Some(id);
    self
  }

  pub(crate) fn name(&mut self, name: String) -> &mut Self {
    self.name = Some(name);
    self
  }
}
