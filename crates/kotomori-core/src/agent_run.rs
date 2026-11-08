use super::*;

#[derive(Debug)]
#[must_use]
pub struct AgentRun {
  event_receiver: UnboundedReceiver<AgentEvent>,
  task: task::JoinHandle<Result>,
}

impl AgentRun {
  pub fn cancel(&self) {
    self.task.abort();
  }

  pub async fn cancel_and_join(mut self) -> (Vec<AgentEvent>, Result) {
    self.cancel();

    let result = match (&mut self.task).await {
      Ok(result) => result,
      Err(error) if error.is_cancelled() => Ok(()),
      Err(error) => Err(error).context("agent task failed"),
    };

    let mut events = Vec::new();

    while let Ok(event) = self.event_receiver.try_recv() {
      events.push(event);
    }

    (events, result)
  }

  pub(super) fn new(
    event_receiver: UnboundedReceiver<AgentEvent>,
    task: task::JoinHandle<Result>,
  ) -> Self {
    Self {
      event_receiver,
      task,
    }
  }

  pub async fn recv(&mut self) -> Option<AgentEvent> {
    self.event_receiver.recv().await
  }
}

impl Drop for AgentRun {
  fn drop(&mut self) {
    self.cancel();
  }
}
