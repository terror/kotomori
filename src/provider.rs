use super::*;

#[derive(Debug, Clone)]
pub(crate) struct CompletionRequest {
  messages: Vec<Message>,
  model: Model,
}

impl CompletionRequest {
  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn model(&self) -> &Model {
    &self.model
  }

  pub(crate) fn new(model: Model, messages: Vec<Message>) -> Self {
    Self { messages, model }
  }
}

#[derive(Debug, Clone)]
pub(crate) enum Provider {
  Fake(Fake),
  Ollama(Ollama),
}

impl Provider {
  pub(crate) fn new(model: &Model) -> Self {
    match model.provider() {
      ProviderName::Fake => Self::Fake(Fake),
      ProviderName::Ollama => Self::Ollama(Ollama::new()),
    }
  }

  pub(crate) async fn stream(
    &self,
    request: CompletionRequest,
    sink: Sink,
  ) -> Result {
    match self {
      Self::Fake(provider) => provider.stream(request, sink).await,
      Self::Ollama(provider) => provider.stream(request, sink).await,
    }
  }
}

#[derive(Debug, Clone)]
pub(crate) struct Sink {
  event_sender: UnboundedSender<Event>,
}

impl Sink {
  pub(crate) fn delta(&self, delta: impl Into<String>) -> Result {
    Ok(self.event_sender.send(Event::AgentDelta(delta.into()))?)
  }

  pub(crate) fn done(&self) -> Result {
    Ok(self.event_sender.send(Event::AgentDone)?)
  }

  pub(crate) fn new(event_sender: UnboundedSender<Event>) -> Self {
    Self { event_sender }
  }
}
