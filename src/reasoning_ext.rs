use super::*;

pub(crate) trait ReasoningExt {
  fn text(&self) -> String;
}

impl ReasoningExt for Reasoning {
  fn text(&self) -> String {
    self
      .content
      .iter()
      .filter_map(|content| match content {
        ReasoningContent::Text { text, .. }
        | ReasoningContent::Summary(text) => Some(text.as_str()),
        _ => None,
      })
      .collect::<Vec<_>>()
      .join("\n")
  }
}

impl ReasoningExt for Sealed<Reasoning> {
  fn text(&self) -> String {
    self.open(self.issuer()).unwrap().text()
  }
}
