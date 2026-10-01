use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct AgentMessage {
  pub(crate) content: Vec<AssistantContent>,
  pub(crate) id: Option<String>,
}

impl From<Vec<AssistantContent>> for AgentMessage {
  fn from(content: Vec<AssistantContent>) -> Self {
    Self { content, id: None }
  }
}
