use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct AgentMessage {
  pub(crate) content: Vec<AssistantContent>,
  pub(crate) id: Option<String>,
}

impl AgentMessage {
  pub(crate) fn text(&self) -> Option<String> {
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
