use super::*;

#[derive(Debug)]
#[must_use]
pub struct AgentRun {
  pub(super) control: AgentControl,
  pub(super) event_receiver: UnboundedReceiver<AgentEvent>,
  pub(super) id: RunId,
  pub(super) state: RunState,
  pub(super) task: task::JoinHandle<Result<RunOutcome>>,
}

impl AgentRun {
  pub fn cancel(&self) {
    self.control.cancel();
  }

  pub async fn cancel_and_join(self) -> RunResult {
    self.cancel();
    self.join().await
  }

  #[must_use]
  pub fn control(&self) -> AgentControl {
    self.control.clone()
  }

  #[must_use]
  pub fn id(&self) -> RunId {
    self.id
  }

  pub async fn join(mut self) -> RunResult {
    self.control.deny_pending();

    while let Some(event) = self.recv().await {
      if let AgentEvent::ToolApprovalRequest(request) = event {
        let _ = self
          .control
          .resolve_approval(&request.id, ToolApproval::Denied);
      }
    }

    let outcome = match (&mut self.task).await {
      Ok(Ok(outcome)) => outcome,
      Ok(Err(error)) => RunOutcome::Failed(format!("{error:#}")),
      Err(error) if error.is_cancelled() => RunOutcome::Cancelled,
      Err(error) => RunOutcome::Failed(format!("agent task failed: {error}")),
    };

    mem::take(&mut self.state).finish(outcome)
  }

  pub async fn recv(&mut self) -> Option<AgentEvent> {
    let event = self.event_receiver.recv().await?;

    self.state.apply(&event);

    Some(event)
  }

  #[must_use]
  pub fn state(&self) -> &RunState {
    &self.state
  }
}

impl Drop for AgentRun {
  fn drop(&mut self) {
    self.cancel();
    self.task.abort();
  }
}
