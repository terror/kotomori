use super::*;

const MAX_TOKENS: u32 = 4096;

pub(crate) struct Anthropic {
  client: anthropic_sdk::Anthropic,
}

impl Anthropic {
  fn message(message: &Message) -> types::MessageParam {
    types::MessageParam {
      content: types::MessageContent::Text(message.content.clone()),
      role: match message.role {
        Role::Agent => types::Role::Assistant,
        Role::User => types::Role::User,
      },
    }
  }

  pub(crate) fn new() -> Result<Self> {
    let base_url = env::var("ANTHROPIC_BASE_URL")
      .unwrap_or_else(|_| "https://api.anthropic.com".into());

    let api_key = env::var("ANTHROPIC_API_KEY")
      .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN"))
      .unwrap_or_default();

    Ok(Self {
      client: anthropic_sdk::Anthropic::with_config(
        anthropic_sdk::ClientConfig::new(api_key)
          .with_base_url(base_url)
          .with_auth_method(AuthMethod::Anthropic),
      )?,
    })
  }

  fn request(request: &Request) -> types::MessageCreateParams {
    types::MessageCreateParams {
      max_tokens: env::var("ANTHROPIC_MAX_TOKENS")
        .ok()
        .and_then(|max_tokens| max_tokens.parse::<u32>().ok())
        .unwrap_or(MAX_TOKENS),
      messages: request.messages().map(Self::message).collect::<Vec<_>>(),
      metadata: None,
      model: request.model_name().into(),
      stop_sequences: None,
      stream: Some(true),
      system: None,
      temperature: None,
      tool_choice: None,
      tools: None,
      top_k: None,
      top_p: None,
    }
  }
}

impl Debug for Anthropic {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Anthropic").finish_non_exhaustive()
  }
}

#[async_trait]
impl Provider for Anthropic {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
    let mut stream = self
      .client
      .messages()
      .create_stream(Self::request(&request))
      .await?;

    while let Some(event) = stream.next().await {
      let event = event?;

      match event {
        types::MessageStreamEvent::ContentBlockDelta {
          delta: types::ContentBlockDelta::TextDelta { text },
          ..
        } if !text.is_empty() => sink.delta(text)?,
        _ => {}
      }
    }

    Ok(())
  }
}
