use super::*;

#[derive(Clone, Debug)]
pub struct ToolContext {
  call: CallId,
  pub(super) cancellation: CancellationToken,
  deadline: Option<Instant>,
  pub(super) event_sender: Arc<Mutex<Option<UnboundedSender<AgentEvent>>>>,
  name: ToolName,
  response: ResponseId,
}

impl ToolContext {
  #[must_use]
  pub fn call(&self) -> &CallId {
    &self.call
  }

  pub async fn cancelled(&self) {
    self.cancellation.cancelled().await;
  }

  #[must_use]
  pub fn deadline(&self) -> Option<Instant> {
    self.deadline
  }

  #[must_use]
  pub fn is_cancelled(&self) -> bool {
    self.cancellation.is_cancelled()
  }

  #[must_use]
  pub fn name(&self) -> &ToolName {
    &self.name
  }

  pub(super) fn new(
    run: &RunContext,
    response: ResponseId,
    call: &ToolRequest,
  ) -> Self {
    Self {
      call: call.id.clone(),
      cancellation: run.control.cancellation.child_token(),
      deadline: run.deadline,
      event_sender: Arc::new(Mutex::new(Some(run.event_sender.clone()))),
      name: call.function.name.clone(),
      response,
    }
  }

  pub fn progress(&self, progress: Value) -> Result {
    self
      .event_sender
      .lock()
      .unwrap()
      .as_ref()
      .context("tool execution has finished")?
      .send(AgentEvent::ToolProgress {
        call: self.call.clone(),
        progress,
        response: self.response,
      })?;

    Ok(())
  }

  #[must_use]
  pub fn response(&self) -> ResponseId {
    self.response
  }
}
