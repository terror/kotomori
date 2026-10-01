use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct ReasoningDraft {
  pub(super) pending: Option<String>,
  pub(super) protocol: Reasoning,
}

impl ReasoningDraft {
  pub(crate) fn append(&mut self, mut reasoning: Reasoning) {
    if let Some(ReasoningContent::Text { text, .. }) = reasoning
      .content
      .iter_mut()
      .find(|content| matches!(content, ReasoningContent::Text { .. }))
      && let Some(pending) = self.pending.take()
    {
      text.insert_str(0, &pending);
    }

    self.snapshot(reasoning);
  }

  pub(crate) fn finish(self) -> Option<Reasoning> {
    if self.protocol.content.is_empty() {
      self
        .pending
        .map(|pending| Reasoning::new(&pending).optional_id(self.protocol.id))
    } else {
      Some(self.protocol)
    }
  }

  pub(crate) fn message(&self) -> Option<Reasoning> {
    (!self.protocol.content.is_empty()).then(|| self.protocol.clone())
  }

  pub(crate) fn preview(&self) -> Reasoning {
    let mut reasoning = self.protocol.clone();

    if let Some(pending) = &self.pending {
      reasoning.content.push(ReasoningContent::Text {
        text: pending.clone(),
        signature: None,
      });
    }

    reasoning
  }

  pub(crate) fn snapshot(&mut self, reasoning: Reasoning) {
    if reasoning.content.iter().any(|content| {
      matches!(
        content,
        ReasoningContent::Text { .. } | ReasoningContent::Summary(_)
      )
    }) {
      self.pending = None;
    }

    self.protocol.id = reasoning.id;
    self.protocol.content.extend(reasoning.content);
  }
}
