use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  model: Model,
  provider: Provider,
}

impl Agent {
  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    model: Model,
  ) -> Self {
    let provider = Provider::new(&model);

    Self {
      event_sender,
      model,
      provider,
    }
  }

  pub(crate) fn spawn(&self, messages: Vec<Message>) {
    let agent = self.clone();

    tokio::spawn(async move {
      if let Err(error) = agent.stream(messages).await {
        let _ = agent.event_sender.send(Event::Error(error.to_string()));
      }
    });
  }

  async fn stream(&self, messages: Vec<Message>) -> Result {
    self
      .provider
      .stream(
        CompletionRequest::new(self.model.clone(), messages),
        Sink::new(self.event_sender.clone()),
      )
      .await
  }
}
