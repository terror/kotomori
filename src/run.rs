use super::*;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Run {
  pub(crate) activity: AgentActivity,
  pub(crate) approval: Option<ApprovalRequest>,
  pub(crate) content: Vec<AgentMessageContent>,
  pub(crate) elapsed: Duration,
  pub(crate) frame: usize,
  pub(crate) id: u64,
}

impl Run {
  pub(crate) fn finish(mut self) -> Option<Message> {
    match self.activity {
      AgentActivity::Reasoning(reasoning) if !reasoning.is_empty() => {
        self.content.push(AgentMessageContent::Reasoning(reasoning));
      }
      AgentActivity::Streaming(message) if !message.is_empty() => {
        self.content.push(AgentMessageContent::Text(message));
      }
      AgentActivity::Reasoning(_)
      | AgentActivity::Streaming(_)
      | AgentActivity::Waiting => {}
    }

    (!self.content.is_empty()).then_some(Message::Agent(self.content))
  }

  pub(crate) fn new(id: u64) -> Self {
    Self {
      activity: AgentActivity::Waiting,
      approval: None,
      content: Vec::new(),
      elapsed: Duration::ZERO,
      frame: 0,
      id,
    }
  }

  pub(crate) fn push_delta(&mut self, delta: &str) {
    self.activity = match mem::take(&mut self.activity) {
      AgentActivity::Waiting if delta.is_empty() => AgentActivity::Waiting,
      AgentActivity::Waiting => AgentActivity::Streaming(delta.into()),
      AgentActivity::Reasoning(reasoning) => {
        if !reasoning.is_empty() {
          self.content.push(AgentMessageContent::Reasoning(reasoning));
        }

        if delta.is_empty() {
          AgentActivity::Waiting
        } else {
          AgentActivity::Streaming(delta.into())
        }
      }
      AgentActivity::Streaming(mut message) => {
        message.push_str(delta);
        AgentActivity::Streaming(message)
      }
    };
  }

  pub(crate) fn push_reasoning_delta(&mut self, delta: &str) {
    self.activity = match mem::take(&mut self.activity) {
      AgentActivity::Waiting if delta.is_empty() => AgentActivity::Waiting,
      AgentActivity::Waiting => AgentActivity::Reasoning(delta.into()),
      AgentActivity::Reasoning(mut reasoning) => {
        reasoning.push_str(delta);
        AgentActivity::Reasoning(reasoning)
      }
      AgentActivity::Streaming(message) => {
        if !message.is_empty() {
          self.content.push(AgentMessageContent::Text(message));
        }

        if delta.is_empty() {
          AgentActivity::Waiting
        } else {
          AgentActivity::Reasoning(delta.into())
        }
      }
    };
  }

  pub(crate) fn reset_message(&mut self) {
    self.activity = AgentActivity::Waiting;
    self.approval = None;
    self.content.clear();
  }

  pub(crate) fn resolve_approval(&mut self, approval: ToolApproval) {
    let Some(request) = self.approval.take() else {
      return;
    };

    match approval {
      ToolApproval::Approved => request.approve(),
      ToolApproval::Denied => request.deny(),
    }
  }

  pub(crate) fn tick(&mut self, elapsed: Duration) {
    self.elapsed = self.elapsed.saturating_add(elapsed);
    self.frame = self.frame.wrapping_add(1);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn finish_ignores_empty_content() {
    for activity in [
      AgentActivity::Waiting,
      AgentActivity::Reasoning(String::new()),
      AgentActivity::Streaming(String::new()),
    ] {
      let run = Run {
        activity,
        ..Run::new(0)
      };

      assert_eq!(run.finish(), None);
    }
  }

  #[test]
  fn finish_preserves_content() {
    let mut run = Run::new(0);

    run.push_reasoning_delta("foo");
    run.push_reasoning_delta("bar");
    run.push_delta("baz");
    run.push_delta("qux");
    run.push_reasoning_delta("quux");

    assert_eq!(
      run.finish(),
      Some(Message::Agent(vec![
        AgentMessageContent::Reasoning("foobar".into()),
        AgentMessageContent::Text("bazqux".into()),
        AgentMessageContent::Reasoning("quux".into()),
      ]))
    );
  }

  #[test]
  fn push_delta_empty_preserves_reasoning() {
    let mut run = Run {
      activity: AgentActivity::Reasoning("foo".into()),
      ..Run::new(0)
    };

    run.push_delta("");

    assert_eq!(
      run,
      Run {
        content: vec![AgentMessageContent::Reasoning("foo".into())],
        ..Run::new(0)
      }
    );
  }

  #[test]
  fn push_reasoning_delta_empty_preserves_text() {
    let mut run = Run {
      activity: AgentActivity::Streaming("foo".into()),
      ..Run::new(0)
    };

    run.push_reasoning_delta("");

    assert_eq!(
      run,
      Run {
        content: vec![AgentMessageContent::Text("foo".into())],
        ..Run::new(0)
      }
    );
  }

  #[test]
  fn push_empty_deltas_preserve_waiting() {
    let mut run = Run::new(0);

    run.push_delta("");
    run.push_reasoning_delta("");

    assert_eq!(run, Run::new(0));
  }

  #[test]
  fn reset_message_preserves_elapsed_and_frame() {
    let mut run = Run {
      activity: AgentActivity::Streaming("bar".into()),
      content: vec![AgentMessageContent::Reasoning("foo".into())],
      elapsed: Duration::from_secs(1),
      frame: 1,
      ..Run::new(0)
    };

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
