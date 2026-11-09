use super::*;

pub(super) struct ResponseGuard {
  event_sender: UnboundedSender<AgentEvent>,
  result: ResponseResult,
}

impl ResponseGuard {
  pub(super) fn finish(
    mut self,
    response: Result<ProviderResponse>,
  ) -> Result<ProviderResponse> {
    let response = response.and_then(|response| {
      self
        .result
        .finish_reason
        .clone_from(&response.finish_reason);
      self.result.usage = response.usage;

      response.validate()?;

      Ok(response)
    });

    self.result.outcome = match &response {
      Ok(response) => match response
        .finish_reason
        .as_ref()
        .and_then(FinishReason::outcome)
      {
        Some(RunOutcome::Failed(error)) => ResponseOutcome::Failed(error),
        _ => ResponseOutcome::Completed,
      },
      Err(error) => ResponseOutcome::Failed(format!("{error:#}")),
    };

    response
  }

  pub(super) fn new(
    event_sender: &UnboundedSender<AgentEvent>,
    id: ResponseId,
  ) -> Result<Self> {
    event_sender.send(AgentEvent::ResponseStarted(id))?;

    Ok(Self {
      event_sender: event_sender.clone(),
      result: ResponseResult {
        finish_reason: None,
        id,
        outcome: ResponseOutcome::Interrupted,
        usage: Usage::default(),
      },
    })
  }
}

impl Drop for ResponseGuard {
  fn drop(&mut self) {
    if thread::panicking() {
      self.result.outcome = ResponseOutcome::Failed("provider panicked".into());
    }

    let _ = self
      .event_sender
      .send(AgentEvent::ResponseFinished(self.result.clone()));
  }
}
