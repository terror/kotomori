use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AgentMessage {
  pub content: Vec<AssistantContent>,
  pub id: Option<String>,
}

impl AgentMessage {
  #[must_use]
  pub fn text(&self) -> Option<String> {
    let text = self
      .content
      .iter()
      .filter_map(|content| match content {
        AssistantContent::Text(text) if !text.text.is_empty() => {
          Some(text.text.as_str())
        }
        _ => None,
      })
      .collect::<Vec<_>>()
      .join("\n\n");

    (!text.is_empty()).then_some(text)
  }
}

impl From<Vec<AssistantContent>> for AgentMessage {
  fn from(content: Vec<AssistantContent>) -> Self {
    Self { content, id: None }
  }
}
