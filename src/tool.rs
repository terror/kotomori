use super::*;

#[derive(Debug)]
pub(crate) struct Tool {
  pub(crate) description: &'static str,
  pub(crate) invocation: fn(RawToolCall) -> Result<ToolInvocationKind>,
  pub(crate) name: &'static str,
  pub(crate) parameters: Value,
}

impl Tool {
  pub(crate) fn invocation(
    &self,
    call: RawToolCall,
  ) -> Result<ToolInvocationKind> {
    (self.invocation)(call)
  }

  pub(crate) fn new<T>() -> Self
  where
    T: ToolSpec,
  {
    Self {
      description: T::DESCRIPTION,
      invocation: ToolInvocation::from_raw::<T>,
      name: T::NAME,
      parameters: serde_json::to_value(T::json_schema(
        &mut schemars::SchemaGenerator::default(),
      ))
      .expect("failed to serialize tool schema"),
    }
  }
}

impl From<&Tool> for ToolDefinition {
  fn from(tool: &Tool) -> Self {
    Self {
      description: tool.description.into(),
      name: tool.name.into(),
      parameters: tool.parameters.clone(),
    }
  }
}
