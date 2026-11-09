use super::*;

pub(super) struct ToolGuard {
  context: ToolContext,
  result: Option<ToolResult>,
}

impl ToolGuard {
  pub(super) fn finish(mut self, result: &ToolResult) {
    self.result = Some(result.clone());
  }

  pub(super) fn new(context: ToolContext) -> Result<Self> {
    context
      .event_sender
      .lock()
      .unwrap()
      .as_ref()
      .context("tool execution has finished")?
      .send(AgentEvent::ToolStarted {
        call: context.call().clone(),
        name: context.name().clone(),
        response: context.response(),
      })?;

    Ok(Self {
      context,
      result: None,
    })
  }
}

impl Drop for ToolGuard {
  fn drop(&mut self) {
    let interrupted = self.result.is_none();

    let result = self.result.take().unwrap_or_else(|| {
      let (outcome, stderr) = if thread::panicking() {
        (ToolOutcome::Failure, "tool panicked")
      } else {
        (ToolOutcome::Interrupted, "interrupted")
      };

      ToolResult {
        content: stderr.into(),
        outcome,
        ..Default::default()
      }
    });

    self.context.cancellation.cancel();

    if let Some(event_sender) = self.context.event_sender.lock().unwrap().take()
    {
      let message = interrupted.then(|| {
        Message::User(vec![UserMessageContent::ToolResult {
          call: self.context.call().clone(),
          name: self.context.name().clone(),
          result: result.clone(),
        }])
      });

      let _ = event_sender.send(AgentEvent::ToolFinished {
        call: self.context.call().clone(),
        response: self.context.response(),
        result,
      });

      if let Some(message) = message {
        let _ = event_sender.send(AgentEvent::Message(message));
      }
    }
  }
}
