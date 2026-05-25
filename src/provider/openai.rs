use super::*;

#[derive(Debug, Clone)]
pub(crate) struct OpenAi {
  client: async_openai::Client<OpenAIConfig>,
}

impl OpenAi {
  fn message(message: &Message) -> ChatCompletionRequestMessage {
    match message.role {
      Role::Agent => ChatCompletionRequestMessage::Assistant(
        ChatCompletionRequestAssistantMessage {
          content: Some(ChatCompletionRequestAssistantMessageContent::Text(
            message.content.clone(),
          )),
          ..Default::default()
        },
      ),
      Role::User => {
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
          content: ChatCompletionRequestUserMessageContent::Text(
            message.content.clone(),
          ),
          name: None,
        })
      }
    }
  }

  pub(crate) fn new() -> Self {
    Self::with_config(OpenAIConfig::new())
  }

  fn request(request: &Request) -> Result<CreateChatCompletionRequest> {
    Ok(
      CreateChatCompletionRequestArgs::default()
        .model(request.model_name())
        .messages(request.messages().map(Self::message).collect::<Vec<_>>())
        .build()?,
    )
  }

  pub(crate) fn with_config(config: OpenAIConfig) -> Self {
    Self {
      client: async_openai::Client::with_config(config),
    }
  }
}

#[async_trait]
impl Provider for OpenAi {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
    let mut stream = self
      .client
      .chat()
      .create_stream(Self::request(&request)?)
      .await?;

    while let Some(response) = stream.next().await {
      let response = response?;

      for content in response
        .choices
        .into_iter()
        .filter_map(|choice| choice.delta.content)
        .filter(|content| !content.is_empty())
      {
        sink.delta(content)?;
      }
    }

    Ok(())
  }
}
