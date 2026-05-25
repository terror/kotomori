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
