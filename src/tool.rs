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

impl From<&Tool> for anthropic::Tool {
  fn from(tool: &Tool) -> Self {
    let Value::Object(schema) = tool.parameters.clone() else {
      unreachable!()
    };

    let properties = schema
      .get("properties")
      .and_then(Value::as_object)
      .cloned()
      .unwrap_or_default();

    let required = schema
      .get("required")
      .and_then(Value::as_array)
      .into_iter()
      .flatten()
      .filter_map(Value::as_str)
      .map(str::to_string)
      .collect();

    let additional = schema
      .into_iter()
      .filter(|(key, _)| {
        key != "properties" && key != "required" && key != "type"
      })
      .collect();

    Self {
      description: tool.description.into(),
      input_schema: anthropic::ToolInputSchema {
        additional,
        properties,
        required,
        schema_type: "object".into(),
      },
      name: tool.name.into(),
    }
  }
}

impl TryFrom<&Tool> for ollama::ToolInfo {
  type Error = Error;

  fn try_from(tool: &Tool) -> Result<Self> {
    Ok(Self {
      function: ollama::ToolFunctionInfo {
        description: tool.description.into(),
        name: tool.name.into(),
        parameters: serde_json::from_value::<schemars::Schema>(
          tool.parameters.clone(),
        )?,
      },
      tool_type: ollama::ToolType::Function,
    })
  }
}

impl From<&Tool> for openai::ChatCompletionTools {
  fn from(tool: &Tool) -> Self {
    Self::Function(openai::ChatCompletionTool {
      function: openai::FunctionObject {
        description: Some(tool.description.into()),
        name: tool.name.into(),
        parameters: Some(tool.parameters.clone()),
        strict: None,
      },
    })
  }
}
