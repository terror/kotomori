use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AssistantContent {
  Image(Value),
  Reasoning(Reasoning),
  Text(Text),
  ToolCall(ToolRequest),
}

impl AssistantContent {
  #[must_use]
  pub fn reasoning(issuer: &str, text: &str) -> Self {
    Self::Reasoning(Reasoning::new(
      issuer,
      vec![ReasoningContent::Text {
        text: text.into(),
        signature: None,
      }],
    ))
  }

  pub fn text(text: impl Into<String>) -> Self {
    Self::Text(Text {
      additional_params: None,
      text: text.into(),
    })
  }

  #[must_use]
  pub fn tool_call(id: &str, name: ToolName, arguments: Value) -> Self {
    Self::ToolCall(ToolRequest::new(
      CallId::new(id),
      ToolFunction { arguments, name },
    ))
  }
}
