use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Reasoning {
  pub(crate) content: Vec<ReasoningContent>,
  pub(crate) id: Option<String>,
  pub(crate) issuer: String,
}

impl Reasoning {
  pub(crate) fn new(issuer: &str, content: Vec<ReasoningContent>) -> Self {
    Self {
      content,
      id: None,
      issuer: issuer.into(),
    }
  }

  pub(crate) fn text(&self) -> String {
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
