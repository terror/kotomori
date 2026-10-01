use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct MessageBuffer {
  blocks: Vec<MessageDraft>,
  id: Option<String>,
}

impl MessageBuffer {
  pub(crate) fn apply(&mut self, update: MessageUpdate) {
    match update {
      MessageUpdate::MessageId(id) => self.id = Some(id),
      MessageUpdate::Reasoning(reasoning) => self.reasoning(reasoning, false),
      MessageUpdate::ReasoningAppend(reasoning) => {
        self.reasoning(reasoning, true);
      }
      MessageUpdate::ReasoningDelta { delta, id } => {
        self.reasoning_delta(id, &delta);
      }
      MessageUpdate::Text(text) => {
        if text.is_empty() {
          return;
        }

        match self.blocks.last_mut() {
          Some(MessageDraft::Content(AssistantContent::Text(existing))) => {
            existing.text.push_str(&text);
          }
          _ => self
            .blocks
            .push(MessageDraft::Content(AssistantContent::text(text))),
        }
      }
      MessageUpdate::ToolCall(tool_call) => {
        self
          .blocks
          .push(MessageDraft::Content(AssistantContent::ToolCall(tool_call)));
      }
    }
  }

  #[cfg(test)]
  pub(crate) fn apply_many(&mut self, updates: &[MessageUpdate]) {
    for update in updates {
      self.apply(update.clone());
    }
  }

  pub(crate) fn finish(self) -> AgentMessage {
    AgentMessage {
      content: self
        .blocks
        .into_iter()
        .filter_map(|block| match block {
          MessageDraft::Content(content) => Some(content),
          MessageDraft::Reasoning(draft) => {
            draft.finish().map(AssistantContent::Reasoning)
          }
        })
        .collect(),
      id: self.id,
    }
  }

  pub(crate) fn has_pending_reasoning(&self) -> bool {
    self.blocks.iter().any(|block| {
      matches!(block, MessageDraft::Reasoning(draft) if draft.pending.is_some())
    })
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.blocks.is_empty()
  }

  pub(crate) fn message(&self) -> AgentMessage {
    AgentMessage {
      content: self
        .blocks
        .iter()
        .filter_map(|block| match block {
          MessageDraft::Content(content) => Some(content.clone()),
          MessageDraft::Reasoning(draft) => {
            draft.message().map(AssistantContent::Reasoning)
          }
        })
        .collect(),
      id: self.id.clone(),
    }
  }

  pub(crate) fn preview(&self) -> Vec<AssistantContent> {
    self
      .blocks
      .iter()
      .map(|block| match block {
        MessageDraft::Content(content) => content.clone(),
        MessageDraft::Reasoning(draft) => {
          AssistantContent::Reasoning(draft.preview())
        }
      })
      .collect()
  }

  fn reasoning(&mut self, reasoning: Reasoning, append: bool) {
    if reasoning.content.is_empty() {
      return;
    }

    let anonymous = reasoning.id.is_some()
      || reasoning.content.iter().any(|content| {
        matches!(
          content,
          ReasoningContent::Text { .. } | ReasoningContent::Summary(_)
        )
      });

    let id = reasoning.id.as_ref().filter(|id| {
      self.blocks.iter().any(|block| {
        matches!(block, MessageDraft::Reasoning(draft) if draft.protocol.id.as_ref() == Some(*id))
      })
    });

    let draft = self.blocks.iter_mut().rev().find_map(|block| match block {
      MessageDraft::Reasoning(draft)
        if draft.protocol.id.as_ref() == id
          && (id.is_some() || (anonymous && draft.pending.is_some())) =>
      {
        Some(draft)
      }
      _ => None,
    });

    if let Some(draft) = draft {
      if append {
        draft.append(reasoning);
      } else {
        draft.snapshot(reasoning);
      }
    } else {
      self.blocks.push(MessageDraft::Reasoning(ReasoningDraft {
        pending: None,
        protocol: reasoning,
      }));
    }
  }

  fn reasoning_delta(&mut self, id: Option<String>, delta: &str) {
    if delta.is_empty() {
      return;
    }

    let draft = if id.is_some() {
      self.blocks.iter_mut().rev().find_map(|block| match block {
        MessageDraft::Reasoning(draft) if draft.protocol.id == id => {
          Some(draft)
        }
        _ => None,
      })
    } else {
      match self.blocks.last_mut() {
        Some(MessageDraft::Reasoning(draft))
          if draft.protocol.id.is_none() && draft.pending.is_some() =>
        {
          Some(draft)
        }
        _ => None,
      }
    };

    if let Some(draft) = draft {
      draft.pending.get_or_insert_default().push_str(delta);
    } else {
      self.blocks.push(MessageDraft::Reasoning(ReasoningDraft {
        pending: Some(delta.into()),
        protocol: Reasoning::multi(Vec::new()).optional_id(id),
      }));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anonymous_signed_blocks_remain_separate() {
    let mut buffer = MessageBuffer::default();

    let foo = Reasoning::new_with_signature("fooé", Some("bar".into()));
    let baz = Reasoning::new_with_signature("baz", Some("qux".into()));

    for delta in ["foo", "é"] {
      buffer.apply(MessageUpdate::ReasoningDelta {
        delta: delta.into(),
        id: None,
      });
    }

    buffer.apply_many(&[
      MessageUpdate::Reasoning(Reasoning::redacted("quuz")),
      MessageUpdate::Text("quux".into()),
      MessageUpdate::Reasoning(foo.clone()),
      MessageUpdate::ReasoningDelta {
        delta: "baz".into(),
        id: None,
      },
      MessageUpdate::Reasoning(baz.clone()),
    ]);

    let content = vec![
      AssistantContent::Reasoning(foo),
      AssistantContent::Reasoning(Reasoning::redacted("quuz")),
      AssistantContent::text("quux"),
      AssistantContent::Reasoning(baz),
    ];

    assert!(!buffer.has_pending_reasoning());
    assert_eq!(buffer.preview(), content);
    assert_eq!(buffer.finish(), AgentMessage { content, id: None });
  }

  #[test]
  fn anonymous_summaries_adopt_final_identifier() {
    let mut buffer = MessageBuffer::default();

    let summary =
      Reasoning::summaries(vec!["foo".into()]).with_id("bar".into());

    buffer.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "foofoo".into(),
        id: None,
      },
      MessageUpdate::Reasoning(summary.clone()),
      MessageUpdate::Reasoning(summary),
      MessageUpdate::Reasoning(
        Reasoning::encrypted("baz").with_id("bar".into()),
      ),
    ]);

    let mut reasoning = Reasoning::summaries(vec!["foo".into(), "foo".into()])
      .with_id("bar".into());

    reasoning
      .content
      .push(ReasoningContent::Encrypted("baz".into()));

    let content = vec![AssistantContent::Reasoning(reasoning)];

    assert!(!buffer.has_pending_reasoning());

    assert_eq!(buffer.message().content, content);
    assert_eq!(buffer.preview(), content);
    assert_eq!(buffer.finish(), AgentMessage { content, id: None });
  }

  #[test]
  fn deltas_promote_only_after_success() {
    let mut buffer = MessageBuffer::default();

    buffer.apply_many(&[
      MessageUpdate::Text(String::new()),
      MessageUpdate::ReasoningDelta {
        delta: String::new(),
        id: None,
      },
    ]);

    assert_eq!(buffer, MessageBuffer::default());

    assert!(buffer.is_empty());

    for delta in ["foo", "é"] {
      buffer.apply(MessageUpdate::ReasoningDelta {
        delta: delta.into(),
        id: None,
      });
    }

    buffer.apply_many(&[
      MessageUpdate::Text("bar".into()),
      MessageUpdate::Text("baz".into()),
      MessageUpdate::MessageId("qux".into()),
    ]);

    assert!(!buffer.is_empty());
    assert!(buffer.has_pending_reasoning());
    assert_eq!(
      buffer.message(),
      AgentMessage {
        content: vec![AssistantContent::text("barbaz")],
        id: Some("qux".into()),
      }
    );

    let content = vec![
      AssistantContent::reasoning("fooé"),
      AssistantContent::text("barbaz"),
    ];

    assert_eq!(buffer.preview(), content);
    assert_eq!(
      serde_json::from_str::<MessageBuffer>(
        &serde_json::to_string(&buffer).unwrap()
      )
      .unwrap(),
      buffer
    );
    assert_eq!(
      buffer.finish(),
      AgentMessage {
        content,
        id: Some("qux".into()),
      }
    );
  }

  #[test]
  fn identified_drafts_allow_interleaving() {
    let mut buffer = MessageBuffer::default();

    for (id, delta) in [("foo", "bar"), ("baz", "qux"), ("foo", "quux")] {
      buffer.apply(MessageUpdate::ReasoningDelta {
        delta: delta.into(),
        id: Some(id.into()),
      });
    }

    let foo = Reasoning::new_with_signature("barquux", Some("quuz".into()))
      .with_id("foo".into());
    let baz = Reasoning::new("qux").with_id("baz".into());

    buffer.apply_many(&[
      MessageUpdate::Reasoning(foo.clone()),
      MessageUpdate::Reasoning(baz.clone()),
    ]);

    assert_eq!(
      buffer.finish(),
      AgentMessage {
        content: vec![
          AssistantContent::Reasoning(foo),
          AssistantContent::Reasoning(baz),
        ],
        id: None,
      }
    );
  }

  #[test]
  fn opaque_protocol_keeps_pending_text_display_only() {
    let mut buffer = MessageBuffer::default();

    let reasoning = Reasoning::encrypted("bar").with_id("baz".into());

    buffer.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        id: None,
      },
      MessageUpdate::Reasoning(reasoning.clone()),
    ]);

    let content = vec![AssistantContent::Reasoning(reasoning.clone())];
    let mut preview = reasoning;
    preview.content.push(ReasoningContent::Text {
      text: "foo".into(),
      signature: None,
    });

    assert!(buffer.has_pending_reasoning());
    assert_eq!(buffer.message().content, content);
    assert_eq!(buffer.preview(), [AssistantContent::Reasoning(preview)]);
    assert_eq!(buffer.finish(), AgentMessage { content, id: None });
  }

  #[test]
  fn signed_final_chunk_appends_to_deltas() {
    let mut buffer = MessageBuffer::default();

    buffer.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "fooé".into(),
        id: None,
      },
      MessageUpdate::ReasoningAppend(Reasoning::new_with_signature(
        "bar",
        Some("baz".into()),
      )),
    ]);

    assert!(!buffer.has_pending_reasoning());
    assert_eq!(
      buffer.finish(),
      AgentMessage {
        content: vec![AssistantContent::Reasoning(
          Reasoning::new_with_signature("fooébar", Some("baz".into()))
        )],
        id: None,
      }
    );
  }
}
