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

    let mut tool_calls = BTreeMap::<u32, PendingToolCall>::new();

    while let Some(response) = stream.next().await {
      for choice in response?.choices {
        if let Some(content) =
          choice.delta.content.filter(|content| !content.is_empty())
        {
          sink.delta(content)?;
        }

        for chunk in choice.delta.tool_calls.into_iter().flatten() {
          tool_calls.entry(chunk.index).or_default().append(chunk);
        }
      }
    }

    for tool_call in tool_calls.into_values() {
      sink.tool_call(tool_call.finish()?)?;
    }

    Ok(())
  }
}
