use super::*;

pub(super) struct RunContext {
  pub(super) control: AgentControl,
  pub(super) deadline: Option<Instant>,
  pub(super) event_sender: UnboundedSender<AgentEvent>,
  pub(super) id: RunId,
}

impl RunContext {
  pub(super) async fn interrupted(&self) -> RunOutcome {
    tokio::select! {
      biased;
      () = self.control.cancellation.cancelled() => RunOutcome::Cancelled,
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
      control: AgentControl::new(),
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

  pub(super) fn send_message(
    &self,
    response: Option<ResponseId>,
    message: Message,
  ) -> Result {
    self.event_sender.send(AgentEvent::Message {
      message,
      response,
      run: self.id,
    })?;

    Ok(())
  }
}

impl Drop for RunContext {
  fn drop(&mut self) {
    self.control.cancel();
  }
}
