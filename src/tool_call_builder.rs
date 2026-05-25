use super::*;

#[derive(Default)]
pub(crate) struct ToolCallBuilder {
  argument_fragments: String,
  arguments: Option<Value>,
  id: Option<String>,
  name: Option<String>,
}

impl ToolCallBuilder {
  pub(crate) fn argument_fragment(
    self,
    argument_fragment: Option<&str>,
  ) -> Self {
    Self {
      argument_fragments: argument_fragment
        .map(|fragment| format!("{}{fragment}", self.argument_fragments))
        .unwrap_or(self.argument_fragments),
      ..self
    }
  }

  pub(crate) fn arguments(self, arguments: Option<Value>) -> Self {
    Self {
      arguments: arguments.or(self.arguments),
      ..self
    }
  }

  pub(crate) fn finish(self) -> Result<RawToolCall> {
    let id = self.id.context("missing tool call id")?;

    let name = self.name.context("missing tool call name")?;

    if self.argument_fragments.trim().is_empty() {
      Ok(RawToolCall::new(
        id,
        name,
        self.arguments.unwrap_or_else(|| json!({})),
      ))
    } else {
      RawToolCall::from_arguments_string(id, name, self.argument_fragments)
    }
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
