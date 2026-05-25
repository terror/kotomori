use super::*;

#[derive(Debug, Clone)]
pub(crate) struct OpenAi {
  client: async_openai::Client<OpenAIConfig>,
}

impl OpenAi {
  pub(crate) fn new() -> Self {
    Self::with_config(OpenAIConfig::new())
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
      .create_stream((&request).try_into()?)
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
