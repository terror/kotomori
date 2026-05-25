use super::*;

pub(crate) struct Anthropic {
  client: anthropic_sdk::Anthropic,
}

impl Anthropic {
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
      .create_stream((&request).into())
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
