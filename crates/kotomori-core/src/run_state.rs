use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunState {
  pub conversation: Conversation,
  pub entries: Vec<RunEntry>,
  pub message: MessageBuffer,
  pub usage: Usage,
}

impl RunState {
  pub fn apply(&mut self, event: &AgentEvent) {
    match event {
      AgentEvent::Message(message) => {
        self.message = MessageBuffer::default();
        self.push(RunEntry::Message(message.clone()));
      }
      AgentEvent::Update(update) => self.message.apply(update.clone()),
      AgentEvent::Usage(usage) => self.usage = *usage,
      AgentEvent::ToolApprovalRequest(_) | AgentEvent::ToolStarted(_) => {}
    }
  }

  #[must_use]
  pub fn finish(mut self, outcome: RunOutcome) -> RunResult {
    let message = mem::take(&mut self.message);

    if !message.is_empty() {
      self.push(if message.has_pending_reasoning() {
        RunEntry::Draft(message)
      } else {
        RunEntry::Message(Message::Agent(message.finish()))
      });
    }

    let outcome = match self.conversation.interrupt_pending_calls() {
      Ok(messages) => {
        self
          .entries
          .extend(messages.into_iter().map(RunEntry::Message));
        outcome
      }
      Err(error) => RunOutcome::Failed(format!("{error:#}")),
    };

    RunResult {
      conversation: self.conversation,
      entries: self.entries,
      outcome,
      usage: self.usage,
    }
  }

  #[must_use]
  pub fn new(conversation: Conversation) -> Self {
    Self {
      conversation,
      ..Default::default()
    }
  }

  fn push(&mut self, entry: RunEntry) {
    if let Some(message) = entry.clone().into_message() {
      self.conversation.push(message);
    } else if matches!(entry, RunEntry::Message(_)) {
      return;
    }

    self.entries.push(entry);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn finish_run_saves_partial_output() {
    let mut state = RunState::default();

    state.apply(&AgentEvent::Update(MessageUpdate::Text {
      delta: "foo".into(),
      index: 0,
    }));

    let message = Message::Agent(vec![AssistantContent::text("foo")].into());

    assert_eq!(
      state.finish(RunOutcome::Completed),
      RunResult {
        conversation: vec![message.clone()].into(),
        entries: vec![RunEntry::Message(message)],
        outcome: RunOutcome::Completed,
        usage: Usage::default(),
      },
    );
  }

  #[test]
  fn unfinished_reasoning_survives_without_entering_requests() {
    #[track_caller]
    fn case(outcome: RunOutcome) {
      let reasoning = Reasoning::new(
        "foo",
        vec![ReasoningContent::Text {
          text: "foo".into(),
          signature: Some("bar".into()),
        }],
      );

      let mut state = RunState::default();

      for update in [
        MessageUpdate::Content {
          index: 0,
          content: AssistantContent::Reasoning(reasoning.clone()),
        },
        MessageUpdate::Text {
          delta: "baz".into(),
          index: 1,
        },
        MessageUpdate::ReasoningDelta {
          delta: "quux".into(),
          index: 2,
        },
      ] {
        state.apply(&AgentEvent::Update(update));
      }

      let draft = state.message.clone();

      assert_eq!(
        state.finish(outcome.clone()),
        RunResult {
          conversation: vec![Message::Agent(
            vec![
              AssistantContent::Reasoning(reasoning),
              AssistantContent::text("baz"),
            ]
            .into(),
          )]
          .into(),
          entries: vec![RunEntry::Draft(draft)],
          outcome,
          usage: Usage::default(),
        },
      );
    }

    case(RunOutcome::Cancelled);
    case(RunOutcome::Failed("foo".into()));
  }
}
