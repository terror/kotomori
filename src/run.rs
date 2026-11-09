use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct Run {
  pub(crate) activity: AgentActivity,
  pub(crate) approval: Option<ApprovalRequest>,
  pub(crate) elapsed: Duration,
  pub(crate) frame: usize,
  pub(crate) id: u64,
  pub(crate) interruption: Option<Interruption>,
  pub(crate) message: MessageBuffer,
  pub(crate) received_entries: usize,
  pub(crate) running_tool: Option<CallId>,
}

impl Run {
  pub(crate) fn new(id: u64) -> Self {
    Self {
      activity: AgentActivity::Waiting,
      approval: None,
      elapsed: Duration::ZERO,
      frame: 0,
      id,
      interruption: None,
      message: MessageBuffer::default(),
      received_entries: 0,
      running_tool: None,
    }
  }

  pub(crate) fn reset_message(&mut self) {
    self.activity = AgentActivity::Waiting;
    self.approval = None;
    self.message = MessageBuffer::default();
    self.running_tool = None;
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
