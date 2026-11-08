use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MessageBuffer {
  blocks: BTreeMap<usize, MessageDraft>,
  id: Option<String>,
}

impl MessageBuffer {
  pub fn apply(&mut self, update: MessageUpdate) {
    match update {
      MessageUpdate::Content { index, content } => {
        self.blocks.insert(index, MessageDraft::Content(content));
      }
      MessageUpdate::MessageId(id) => self.id = Some(id),
      MessageUpdate::ReasoningDelta { delta, index } if !delta.is_empty() => {
        if let MessageDraft::Reasoning(text) = self
          .blocks
          .entry(index)
          .or_insert_with(|| MessageDraft::Reasoning(String::new()))
        {
          text.push_str(&delta);
        }
      }
      MessageUpdate::Text { delta, index } if !delta.is_empty() => {
        if let MessageDraft::Content(AssistantContent::Text(text)) = self
          .blocks
          .entry(index)
          .or_insert_with(|| MessageDraft::Content(AssistantContent::text("")))
        {
          text.text.push_str(&delta);
        }
      }
      MessageUpdate::ReasoningDelta { .. } | MessageUpdate::Text { .. } => {}
    }
  }

  #[cfg(test)]
  pub(crate) fn apply_many(&mut self, updates: &[MessageUpdate]) {
    for update in updates {
      self.apply(update.clone());
    }
  }

  #[must_use]
  pub fn finish(self) -> AgentMessage {
    AgentMessage {
      content: self
        .blocks
        .into_values()
        .filter_map(|block| match block {
          MessageDraft::Content(content) => Some(content),
          MessageDraft::Reasoning(_) => None,
        })
        .collect(),
      id: self.id,
    }
  }

  #[must_use]
  pub fn has_pending_reasoning(&self) -> bool {
    self
      .blocks
      .values()
      .any(|block| matches!(block, MessageDraft::Reasoning(_)))
  }

  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.blocks.is_empty()
  }

  #[must_use]
  pub fn message(&self) -> AgentMessage {
    self.clone().finish()
  }

  pub fn preview(&self) -> impl Iterator<Item = &MessageDraft> {
    self.blocks.values()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn unfinished_reasoning_is_display_only() {
    let mut buffer = MessageBuffer::default();

    buffer.apply_many(&[
      MessageUpdate::Text {
        delta: String::new(),
        index: 0,
      },
      MessageUpdate::ReasoningDelta {
        delta: String::new(),
        index: 1,
      },
    ]);

    assert_eq!(buffer, MessageBuffer::default());

    assert!(buffer.is_empty());

    buffer.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        index: 0,
      },
      MessageUpdate::ReasoningDelta {
        delta: "é".into(),
        index: 0,
      },
      MessageUpdate::Text {
        delta: "bar".into(),
        index: 1,
      },
      MessageUpdate::MessageId("baz".into()),
    ]);

    assert!(!buffer.is_empty());
    assert!(buffer.has_pending_reasoning());

    assert_eq!(
      buffer.preview().cloned().collect::<Vec<_>>(),
      vec![
        MessageDraft::Reasoning("fooé".into()),
        MessageDraft::Content(AssistantContent::text("bar")),
      ]
    );

    assert_eq!(
      serde_json::from_str::<MessageBuffer>(
        &serde_json::to_string(&buffer).unwrap()
      )
      .unwrap(),
      buffer,
    );

    let message = AgentMessage {
      content: vec![AssistantContent::text("bar")],
      id: Some("baz".into()),
    };

    assert_eq!(buffer.message(), message);
    assert_eq!(buffer.finish(), message);
  }
}
