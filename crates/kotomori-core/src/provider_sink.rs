use super::*;

#[derive(Debug)]
pub struct ProviderSink {
  pub(super) event_sender: UnboundedSender<AgentEvent>,
}

impl ProviderSink {
  #[must_use]
  pub fn new(event_sender: UnboundedSender<AgentEvent>) -> Self {
    Self { event_sender }
  }

  pub fn update(&self, mut update: MessageUpdate) -> Result {
    match &mut update {
      MessageUpdate::Complete(message) => {
        message
          .content
          .retain(|content| !matches!(content, AssistantContent::ToolCall(_)));
      }
      MessageUpdate::Content {
        content: AssistantContent::ToolCall(_),
        ..
      } => return Ok(()),
      _ => {}
    }

    self.event_sender.send(AgentEvent::Update(update))?;

    Ok(())
  }

  pub fn update_many(
    &self,
    updates: impl IntoIterator<Item = MessageUpdate>,
  ) -> Result {
    updates
      .into_iter()
      .try_for_each(|update| self.update(update))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn completion_replaces_drafts_without_publishing_tools() {
    let mut buffer = MessageBuffer::default();

    buffer.apply_many(&[
      MessageUpdate::Content {
        content: AssistantContent::reasoning("foo", "bar"),
        index: 2,
      },
      MessageUpdate::ReasoningDelta {
        delta: "baz".into(),
        index: 4,
      },
      MessageUpdate::Text {
        delta: "qux".into(),
        index: 5,
      },
    ]);

    let (event_sender, mut events) = mpsc::unbounded_channel();

    let sink = ProviderSink { event_sender };

    let reasoning = AssistantContent::Reasoning(Reasoning::new(
      "qux",
      vec![ReasoningContent::Text {
        text: "bar".into(),
        signature: Some("baz".into()),
      }],
    ));

    let message = AgentMessage {
      content: vec![
        reasoning.clone(),
        AssistantContent::tool_call(
          "foo",
          ToolName::new("command").unwrap(),
          serde_json::json!({"command": "bar"}),
        ),
      ],
      id: Some("quux".into()),
    };

    sink.update(MessageUpdate::Complete(message)).unwrap();

    let AgentEvent::Update(update) = events.try_recv().unwrap() else {
      panic!();
    };

    let preview = AgentMessage {
      content: vec![reasoning],
      id: Some("quux".into()),
    };

    assert_eq!(update, MessageUpdate::Complete(preview.clone()));

    buffer.apply(update);

    assert_eq!(buffer.finish(), preview);

    assert_eq!(events.try_recv(), Err(mpsc::error::TryRecvError::Empty));
  }
}
