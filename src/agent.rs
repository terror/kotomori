use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  model: String,
}

impl Agent {
  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    model: String,
  ) -> Self {
    Self {
      event_sender,
      model,
    }
  }

  pub(crate) fn spawn(&self, input: String) {
    let agent = self.clone();

    tokio::spawn(async move {
      if let Err(error) = agent.stream(input).await {
        let _ = agent.event_sender.send(Event::Error(error.to_string()));
      }
    });
  }

  async fn stream(&self, input: String) -> Result {
    let response = format!("queued for {}: {input}", self.model);

    for c in response.chars() {
      self.event_sender.send(Event::AgentDelta(c.to_string()))?;
      sleep(Duration::from_millis(20)).await;
    }

    self.event_sender.send(Event::AgentDone)?;

    Ok(())
  }
}
