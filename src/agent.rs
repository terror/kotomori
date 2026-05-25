use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  model: Model,
  provider: Arc<dyn Provider>,
}

impl Agent {
  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    model: Model,
  ) -> Result<Self> {
    let provider = Arc::<dyn Provider>::try_from(model.clone())?;

    Ok(Self {
      event_sender,
      model,
      provider,
    })
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
        Request::new(self.model.clone(), messages),
        ProviderSink::new(self.event_sender.clone()),
      )
      .await?;

    self.event_sender.send(Event::AgentDone)?;

    Ok(())
  }
}
