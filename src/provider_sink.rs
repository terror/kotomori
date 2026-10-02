use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  pub(super) event_sender: UnboundedSender<Event>,
  pub(super) run_id: u64,
}

impl ProviderSink {
  pub(crate) fn update(&self, mut update: MessageUpdate) -> Result {
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

    self.event_sender.send(Event::Agent {
      event: AgentEvent::Update(update),
      run_id: self.run_id,
    })?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn completion_replaces_drafts_without_publishing_tools() {
    let mut run = Run::new(0);

    run.update_many(&[
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

    let sink = ProviderSink {
      event_sender,
      run_id: 0,
    };

    let reasoning = AssistantContent::Reasoning(
      Reasoning::new_with_signature("bar", Some("baz".into())).sealed("qux"),
    );

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

    let Event::Agent {
      event: AgentEvent::Update(update),
      run_id: 0,
    } = events.try_recv().unwrap()
    else {
      panic!();
    };

    let preview = AgentMessage {
      content: vec![reasoning],
      id: Some("quux".into()),
    };

    assert_eq!(update, MessageUpdate::Complete(preview.clone()));

    run.update(update);

    assert_eq!(
      run.finish(),
      Some(TranscriptEntry::Message(Message::Agent(preview)))
    );

    assert_eq!(events.try_recv(), Err(mpsc::error::TryRecvError::Empty));
  }
}
