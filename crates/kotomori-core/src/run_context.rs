use super::*;

pub(super) struct RunContext {
  pub(super) cancellation: CancellationToken,
  pub(super) deadline: Option<Instant>,
  pub(super) event_sender: UnboundedSender<AgentEvent>,
  pub(super) id: RunId,
}

impl RunContext {
  pub(super) async fn interrupted(&self) -> RunOutcome {
    tokio::select! {
      biased;
      () = self.cancellation.cancelled() => RunOutcome::Cancelled,
      () = async {
        match self.deadline {
          Some(deadline) => sleep_until(deadline.into()).await,
          None => pending().await,
        }
      } => RunOutcome::Limited(RunLimit::Deadline),
    }
  }

  pub(super) fn new(
    event_sender: UnboundedSender<AgentEvent>,
    deadline: Option<Instant>,
  ) -> Self {
    Self {
      cancellation: CancellationToken::new(),
      deadline,
      event_sender,
      id: RunId::new(),
    }
  }

  pub(super) fn response(&self, round: usize) -> ResponseId {
    ResponseId {
      round,
      run: self.id,
    }
  }
}
