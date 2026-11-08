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

  pub fn update(&self, update: MessageUpdate) -> Result {
    if matches!(
      update,
      MessageUpdate::Content {
        content: AssistantContent::ToolCall(_),
        ..
      }
    ) {
      return Ok(());
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
  fn finalized_content_replaces_drafts_without_publishing_tools() {
    let (event_sender, mut events) = mpsc::unbounded_channel();

    let sink = ProviderSink { event_sender };

    let reasoning = AssistantContent::Reasoning(Reasoning::new(
      "qux",
      vec![ReasoningContent::Text {
        text: "bar".into(),
        signature: Some("baz".into()),
      }],
    ));

    let updates = [
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        index: 2,
      },
      MessageUpdate::Content {
        content: reasoning.clone(),
        index: 2,
      },
      MessageUpdate::Text {
        delta: "qux".into(),
        index: 5,
      },
      MessageUpdate::MessageId("quux".into()),
    ];

    sink.update_many(updates.clone()).unwrap();

    sink
      .update(MessageUpdate::Content {
        content: AssistantContent::tool_call(
          "foo",
          ToolName::new("command").unwrap(),
          serde_json::json!({"command": "bar"}),
        ),
        index: 4,
      })
      .unwrap();

    let mut buffer = MessageBuffer::default();

    for update in updates {
      assert_eq!(
        events.try_recv().unwrap(),
        AgentEvent::Update(update.clone()),
      );

      buffer.apply(update);
    }

    assert_eq!(
      buffer.finish(),
      AgentMessage {
        content: vec![reasoning, AssistantContent::text("qux")],
        id: Some("quux".into()),
      }
    );

    assert_eq!(events.try_recv(), Err(mpsc::error::TryRecvError::Empty));
  }
}
