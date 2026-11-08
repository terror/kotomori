use super::*;

#[derive(Debug)]
#[must_use]
pub struct AgentRun {
  pub(super) event_receiver: UnboundedReceiver<AgentEvent>,
  pub(super) state: RunState,
  pub(super) task: task::JoinHandle<Result<RunOutcome>>,
}

impl AgentRun {
  pub fn cancel(&self) {
    self.task.abort();
  }

  pub async fn cancel_and_join(self) -> RunResult {
    self.cancel();
    self.join().await
  }

  pub async fn join(mut self) -> RunResult {
    while self.recv().await.is_some() {}

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
}

impl Drop for AgentRun {
  fn drop(&mut self) {
    self.cancel();
  }
}
