use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct Run {
  pub(crate) activity: AgentActivity,
  pub(crate) approval: Option<ApprovalRequest>,
  pub(crate) elapsed: Duration,
  pub(crate) frame: usize,
  pub(crate) id: u64,
  pub(crate) message: MessageBuffer,
}

impl Run {
  pub(crate) fn finish(self) -> Option<TranscriptEntry> {
    if self.message.is_empty() {
      None
    } else if self.message.has_pending_reasoning() {
      Some(TranscriptEntry::Draft(self.message))
    } else {
      Some(TranscriptEntry::Message(Message::Agent(
        self.message.finish(),
      )))
    }
  }

  pub(crate) fn new(id: u64) -> Self {
    Self {
      activity: AgentActivity::Waiting,
      approval: None,
      elapsed: Duration::ZERO,
      frame: 0,
      id,
      message: MessageBuffer::default(),
    }
  }

  pub(crate) fn reset_message(&mut self) {
    self.activity = AgentActivity::Waiting;
    self.approval = None;
    self.message = MessageBuffer::default();
  }

  pub(crate) fn resolve_approval(&mut self, approval: ToolApproval) {
    let Some(request) = self.approval.take() else {
      return;
    };

    request.respond(approval);
  }

  pub(crate) fn tick(&mut self, elapsed: Duration) {
    self.elapsed = self.elapsed.saturating_add(elapsed);
    self.frame = self.frame.wrapping_add(1);
  }

  pub(crate) fn update(&mut self, update: MessageUpdate) {
    match &update {
      MessageUpdate::Text { delta, .. } if !delta.is_empty() => {
        self.activity = AgentActivity::Streaming;
      }
      MessageUpdate::ReasoningDelta { delta, .. } if !delta.is_empty() => {
        self.activity = AgentActivity::Reasoning;
      }
      MessageUpdate::Content {
        content: AssistantContent::Reasoning(reasoning),
        ..
      } if !reasoning.text().is_empty() => {
        self.activity = AgentActivity::Reasoning;
      }
      MessageUpdate::Content {
        content: AssistantContent::ToolCall(_),
        ..
      } => {
        self.activity = AgentActivity::Waiting;
      }
      _ => {}
    }

    self.message.apply(update);
  }

  #[cfg(test)]
  pub(crate) fn update_many(&mut self, updates: &[MessageUpdate]) {
    for update in updates {
      self.update(update.clone());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn finish_ignores_empty_content() {
    assert_eq!(Run::new(0).finish(), None);
  }

  #[test]
  fn reset_message_preserves_elapsed_and_frame() {
    let mut run = Run {
      elapsed: Duration::from_secs(1),
      frame: 1,
      ..Run::new(0)
    };

    run.update_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "bar".into(),
        index: 0,
      },
      MessageUpdate::Text {
        delta: "baz".into(),
        index: 1,
      },
    ]);

    run.reset_message();

    assert_eq!(
      run,
      Run {
        elapsed: Duration::from_secs(1),
        frame: 1,
        ..Run::new(0)
      }
    );
  }

  #[test]
  fn tick_advances_elapsed_and_frame() {
    let mut run = Run::new(0);

    run.tick(Duration::from_secs(1));

    assert_eq!(
      run,
      Run {
        elapsed: Duration::from_secs(1),
        frame: 1,
        ..Run::new(0)
      }
    );
  }

  #[test]
  fn tick_saturates_elapsed_and_wraps_frame() {
    let mut run = Run {
      elapsed: Duration::MAX,
      frame: usize::MAX,
      ..Run::new(0)
    };

    run.tick(Duration::from_secs(1));

    assert_eq!(
      run,
      Run {
        elapsed: Duration::MAX,
        ..Run::new(0)
      }
    );
  }
}
