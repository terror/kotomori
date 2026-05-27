use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Request {
  messages: Vec<Message>,
  model: Model,
  system: Option<String>,
}

impl Request {
  pub(crate) fn last_user_message(&self) -> Option<&Message> {
    self
      .messages()
      .rev()
      .find(|message| message.user_content().is_some())
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
    Self {
      messages,
      model,
      system: None,
    }
  }

  pub(crate) fn system(&self) -> Option<&str> {
    self.system.as_deref()
  }

  pub(crate) fn with_system(
    model: Model,
    messages: Vec<Message>,
    system: impl Into<String>,
  ) -> Self {
    let system = system.into();

    let system = if system.is_empty() {
      None
    } else {
      Some(system)
    };

    Self {
      messages,
      model,
      system,
    }
  }
}

impl From<&Request> for anthropic::MessageCreateParams {
  fn from(request: &Request) -> Self {
    let builder = request.messages().fold(
      anthropic::MessageCreateBuilder::new(
        request.model_name(),
        env::var("ANTHROPIC_MAX_TOKENS")
          .ok()
          .and_then(|max_tokens| max_tokens.parse::<u32>().ok())
          .unwrap_or(4096),
      ),
      |builder, message| {
        let message = anthropic::MessageParam::from(message);

        builder.message(message.role, message.content)
      },
    );

    let builder = if let Some(system) = request.system() {
      builder.system(system)
    } else {
      builder
    };

    builder
      .tools(TOOLS.iter().map(Into::into).collect::<Vec<_>>())
      .build()
  }
}

impl TryFrom<&Request> for openai::CreateChatCompletionRequest {
  type Error = Error;

  fn try_from(request: &Request) -> Result<Self> {
    let system = request.system().map(|system| {
      openai::ChatCompletionRequestMessage::System(
        openai::ChatCompletionRequestSystemMessage {
          content: openai::ChatCompletionRequestSystemMessageContent::Text(
            system.into(),
          ),
          name: None,
        },
      )
    });

    let messages = system
      .into_iter()
      .chain(request.messages().map(Into::into))
      .collect::<Vec<_>>();

    Ok(
      openai::CreateChatCompletionRequestArgs::default()
        .model(request.model_name())
        .messages(messages)
        .tools(TOOLS.iter().map(Into::into).collect::<Vec<_>>())
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
        .map(|message| (message.role(), message.content().unwrap()))
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

    assert_eq!(
      request.last_user_message().unwrap().content().unwrap(),
      "baz"
    );
  }

  #[test]
  fn anthropic_system_context() {
    let request = anthropic::MessageCreateParams::from(&Request::with_system(
      "fake:foo".parse().unwrap(),
      vec![Message::new(Role::User, "bar")],
      "baz",
    ));

    assert_eq!(request.system.as_deref(), Some("baz"));
  }

  #[test]
  fn openai_system_context() {
    let request =
      openai::CreateChatCompletionRequest::try_from(&Request::with_system(
        "fake:foo".parse().unwrap(),
        vec![Message::new(Role::User, "bar")],
        "baz",
      ))
      .unwrap();

    assert_eq!(
      serde_json::to_value(request).unwrap()["messages"],
      json!([
        {
          "role": "system",
          "content": "baz",
        },
        {
          "role": "user",
          "content": "bar",
        },
      ]),
    );
  }

  #[test]
  fn openai_reasoning_effort_can_be_overridden() {
    let mut request =
      openai::CreateChatCompletionRequest::try_from(&Request::new(
        "fake:foo".parse().unwrap(),
        vec![Message::new(Role::User, "bar")],
      ))
      .unwrap();

    request.reasoning_effort = Some(openai::ReasoningEffort::None);

    assert_eq!(
      serde_json::to_value(request).unwrap()["reasoning_effort"],
      "none",
    );
  }
}
