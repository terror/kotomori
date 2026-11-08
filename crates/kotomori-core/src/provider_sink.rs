use super::*;

#[derive(Debug)]
pub struct ProviderSink {
  event_sender: UnboundedSender<RunEvent>,
  message: MessageBuffer,
  message_id: MessageId,
  run_id: RunId,
}

impl ProviderSink {
  pub(crate) fn finish(self) -> MessageBuffer {
    self.message
  }

  #[must_use]
  pub fn new(
    event_sender: UnboundedSender<RunEvent>,
    run_id: RunId,
    message_id: MessageId,
  ) -> Self {
    Self {
      event_sender,
      message: MessageBuffer::default(),
      message_id,
      run_id,
    }
  }

  pub fn update(&mut self, mut update: MessageUpdate) -> Result {
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

    self.message.apply(update.clone());

    self.event_sender.send(RunEvent {
      event: AgentEvent::MessagePreview {
        message_id: self.message_id,
        update,
      },
      run_id: self.run_id,
    })?;

    Ok(())
  }

  pub fn update_many(
    &mut self,
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

    let mut sink = ProviderSink::new(event_sender, RunId(0), MessageId(0));

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

    let preview = AgentMessage {
      content: vec![reasoning],
      id: Some("quux".into()),
    };

    let event = events.try_recv().unwrap();

    assert_eq!(
      event,
      RunEvent {
        event: AgentEvent::MessagePreview {
          message_id: MessageId(0),
          update: MessageUpdate::Complete(preview.clone()),
        },
        run_id: RunId(0),
      }
    );

    let AgentEvent::MessagePreview { update, .. } = event.event else {
      panic!();
    };

    buffer.apply(update);

    assert_eq!(sink.finish(), buffer);
    assert_eq!(buffer.finish(), preview);

    assert_eq!(
      events.try_recv(),
      Err(mpsc::error::TryRecvError::Disconnected)
    );
  }
}
