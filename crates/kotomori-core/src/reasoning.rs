use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Reasoning {
  pub content: Vec<ReasoningContent>,
  pub id: Option<String>,
  pub issuer: String,
}

impl Reasoning {
  #[must_use]
  pub fn new(issuer: &str, content: Vec<ReasoningContent>) -> Self {
    Self {
      content,
      id: None,
      issuer: issuer.into(),
    }
  }

  #[must_use]
  pub fn text(&self) -> String {
    self
      .content
      .iter()
      .filter_map(|content| match content {
        ReasoningContent::Text { text, .. }
        | ReasoningContent::Summary(text) => Some(text.as_str()),
        ReasoningContent::Encrypted(_) | ReasoningContent::Redacted { .. } => {
          None
        }
      })
      .collect::<Vec<_>>()
      .join("\n")
  }
}
