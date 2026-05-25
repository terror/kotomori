use {super::*, async_openai as openai};

#[derive(Debug, Clone)]
pub(crate) struct OpenAi {
  client: openai::Client<openai::config::OpenAIConfig>,
}

impl OpenAi {
  pub(crate) fn new() -> Self {
    Self::with_config(openai::config::OpenAIConfig::new())
  }

  pub(crate) fn with_config(config: openai::config::OpenAIConfig) -> Self {
    Self {
      client: openai::Client::with_config(config),
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

    let mut tool_calls = ToolCallStream::<u32>::default();

    while let Some(response) = stream.next().await {
      for choice in response?.choices {
        if let Some(content) =
          choice.delta.content.filter(|content| !content.is_empty())
        {
          sink.delta(content)?;
        }

        for chunk in choice.delta.tool_calls.into_iter().flatten() {
          tool_calls.push_event(chunk)?;
        }
      }
    }

    for tool_call in tool_calls.finish_all()? {
      sink.tool_call(tool_call)?;
    }

    Ok(())
  }
}
