use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Request {
  messages: Vec<Message>,
  model: Model,
}

impl Request {
  pub(crate) fn last_user_message(&self) -> Option<&Message> {
    self
      .messages()
      .rev()
      .find(|message| message.role == Role::User)
  }

  pub(crate) fn messages(&self) -> impl DoubleEndedIterator<Item = &Message> {
    self.messages.iter()
  }

  pub(crate) fn model(&self) -> &Model {
    &self.model
  }

  pub(crate) fn model_name(&self) -> &str {
    self.model.name()
  }

  pub(crate) fn new(model: Model, messages: Vec<Message>) -> Self {
    Self { messages, model }
  }
}

impl From<&Request> for anthropic::types::MessageCreateParams {
  fn from(request: &Request) -> Self {
    request
      .messages()
      .map(anthropic::types::MessageParam::from)
      .fold(
        anthropic::types::MessageCreateBuilder::new(
          request.model_name(),
          env::var("ANTHROPIC_MAX_TOKENS")
            .ok()
            .and_then(|max_tokens| max_tokens.parse::<u32>().ok())
            .unwrap_or(4096),
        ),
        |builder, message| builder.message(message.role, message.content),
      )
      .tools(
        inventory::iter::<RegisteredTool>
          .into_iter()
          .map(Into::into)
          .collect::<Vec<_>>(),
      )
      .build()
  }
}

impl TryFrom<&Request> for openai::types::chat::CreateChatCompletionRequest {
  type Error = Error;

  fn try_from(request: &Request) -> Result<Self> {
    Ok(
      openai::types::chat::CreateChatCompletionRequestArgs::default()
        .model(request.model_name())
        .messages(request.messages().map(Into::into).collect::<Vec<_>>())
        .tools(
          inventory::iter::<RegisteredTool>
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        )
        .build()?,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chat_messages() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
      ],
    );

    assert_eq!(request.model_name(), "foo");

    assert_eq!(
      request
        .messages()
        .map(|message| (message.role, message.content.as_str()))
        .collect::<Vec<_>>(),
      vec![(Role::User, "foo"), (Role::Agent, "bar")],
    );
  }

  #[test]
  fn last_user_message() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
        Message::new(Role::User, "baz"),
      ],
    );

    assert_eq!(request.last_user_message().unwrap().content, "baz");
  }
}
