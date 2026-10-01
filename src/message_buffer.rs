use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct MessageBuffer {
  blocks: BTreeMap<usize, MessageDraft>,
  id: Option<String>,
}

impl MessageBuffer {
  pub(crate) fn apply(&mut self, update: MessageUpdate) -> bool {
    match update {
      MessageUpdate::Complete(message) => {
        let blocks = message
          .content
          .into_iter()
          .enumerate()
          .map(|(index, content)| (index, MessageDraft::Content(content)))
          .collect();

        let changed = self.blocks != blocks;
        self.blocks = blocks;
        self.id = message.id;
        changed
      }
      MessageUpdate::Content { index, content } => {
        let block = MessageDraft::Content(content);
        let changed = self.blocks.get(&index) != Some(&block);
        self.blocks.insert(index, block);
        changed
      }
      MessageUpdate::MessageId(id) => {
        self.id = Some(id);
        false
      }
      MessageUpdate::ReasoningDelta { delta, index } if !delta.is_empty() => {
        if let MessageDraft::Reasoning(text) = self
          .blocks
          .entry(index)
          .or_insert_with(|| MessageDraft::Reasoning(String::new()))
        {
          text.push_str(&delta);
          true
        } else {
          false
        }
      }
      MessageUpdate::Text { delta, index } if !delta.is_empty() => {
        if let MessageDraft::Content(AssistantContent::Text(text)) = self
          .blocks
          .entry(index)
          .or_insert_with(|| MessageDraft::Content(AssistantContent::text("")))
        {
          text.text.push_str(&delta);
          true
        } else {
          false
        }
      }
      MessageUpdate::ReasoningDelta { .. } | MessageUpdate::Text { .. } => {
        false
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
        .into_values()
        .filter_map(|block| match block {
          MessageDraft::Content(content) => Some(content),
          MessageDraft::Reasoning(_) => None,
        })
        .collect(),
      id: self.id,
    }
  }

  pub(crate) fn has_pending_reasoning(&self) -> bool {
    self
      .blocks
      .values()
      .any(|block| matches!(block, MessageDraft::Reasoning(_)))
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.blocks.is_empty()
  }

  pub(crate) fn message(&self) -> AgentMessage {
    self.clone().finish()
  }

  pub(crate) fn preview(&self) -> impl Iterator<Item = &MessageDraft> {
    self.blocks.values()
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    rig::message::{AdditionalParams, Text},
    serde_json::json,
  };

  #[test]
  fn updates_report_content_changes() {
    let mut buffer = MessageBuffer::default();

    for (update, changed) in [
      (MessageUpdate::MessageId("foo".into()), false),
      (
        MessageUpdate::Text {
          index: 0,
          delta: String::new(),
        },
        false,
      ),
      (
        MessageUpdate::ReasoningDelta {
          index: 0,
          delta: String::new(),
        },
        false,
      ),
      (
        MessageUpdate::Text {
          index: 0,
          delta: "foo".into(),
        },
        true,
      ),
      (
        MessageUpdate::Content {
          index: 0,
          content: AssistantContent::text("foo"),
        },
        false,
      ),
      (
        MessageUpdate::ReasoningDelta {
          index: 0,
          delta: "bar".into(),
        },
        false,
      ),
      (
        MessageUpdate::ReasoningDelta {
          index: 1,
          delta: "bar".into(),
        },
        true,
      ),
      (
        MessageUpdate::Text {
          index: 1,
          delta: "baz".into(),
        },
        false,
      ),
      (
        MessageUpdate::Complete(AgentMessage {
          content: vec![AssistantContent::text("foo")],
          id: None,
        }),
        true,
      ),
      (
        MessageUpdate::Complete(AgentMessage {
          content: vec![AssistantContent::text("foo")],
          id: Some("bar".into()),
        }),
        false,
      ),
    ] {
      assert_eq!(buffer.apply(update), changed);
    }
  }

  #[test]
  fn interleaved_parts_preserve_final_content() {
    let mut buffer = MessageBuffer::default();

    let reasoning = AssistantContent::Reasoning(
      Reasoning {
        content: vec![
          ReasoningContent::Text {
            signature: Some("bar".into()),
            text: "fooé".into(),
          },
          ReasoningContent::Summary("baz".into()),
          ReasoningContent::Encrypted("qux".into()),
          ReasoningContent::Redacted {
            data: "quux".into(),
          },
        ],
        id: Some("quuz".into()),
      }
      .sealed("foo"),
    );

    let tool = AssistantContent::ToolCall(
      rig::message::ToolCall::from_dual_wire(
        "foo",
        "bar",
        ToolFunction {
          arguments: json!({"command": "baz"}),
          name: ToolName::new("command").unwrap(),
        },
      )
      .with_signature(Some("qux".into()))
      .with_additional_params(Some(json!({"foo": "bar"}))),
    );

    let text = AssistantContent::Text(Text {
      additional_params: AdditionalParams::from_entries([(
        "foo",
        json!("bar"),
      )]),
      text: "bazqux".into(),
    });

    buffer.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        index: 0,
      },
      MessageUpdate::Text {
        delta: "baz".into(),
        index: 2,
      },
      MessageUpdate::Text {
        delta: "quux".into(),
        index: 3,
      },
      MessageUpdate::ReasoningDelta {
        delta: "é".into(),
        index: 0,
      },
      MessageUpdate::Text {
        delta: "qux".into(),
        index: 2,
      },
    ]);

    assert_eq!(
      buffer.preview().cloned().collect::<Vec<_>>(),
      vec![
        MessageDraft::Reasoning("fooé".into()),
        MessageDraft::Content(AssistantContent::text("bazqux")),
        MessageDraft::Content(AssistantContent::text("quux")),
      ]
    );

    buffer.apply_many(&[
      MessageUpdate::Content {
        index: 2,
        content: text.clone(),
      },
      MessageUpdate::Content {
        index: 1,
        content: tool.clone(),
      },
      MessageUpdate::Content {
        index: 0,
        content: reasoning.clone(),
      },
      MessageUpdate::Content {
        index: 0,
        content: reasoning.clone(),
      },
      MessageUpdate::MessageId("foo".into()),
    ]);

    let message = AgentMessage {
      content: vec![reasoning, tool, text, AssistantContent::text("quux")],
      id: Some("foo".into()),
    };

    assert!(!buffer.has_pending_reasoning());

    assert_eq!(buffer.message(), message);
    assert_eq!(buffer.finish(), message);
  }

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
