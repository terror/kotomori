use super::*;

#[derive(Debug, Clone)]
pub(crate) struct OpenAi {
  client: crate::openai::Client<crate::openai::OpenAIConfig>,
  reasoning_effort: Option<crate::openai::ReasoningEffort>,
}

impl OpenAi {
  pub(crate) fn new() -> Self {
    Self::with_config(crate::openai::OpenAIConfig::new())
  }

  pub(crate) fn with_config(config: crate::openai::OpenAIConfig) -> Self {
    Self {
      client: crate::openai::Client::with_config(config),
      reasoning_effort: None,
    }
  }

  pub(crate) fn with_reasoning_effort(
    self,
    reasoning_effort: crate::openai::ReasoningEffort,
  ) -> Self {
    Self {
      reasoning_effort: Some(reasoning_effort),
      ..self
    }
  }
}

#[async_trait]
impl Provider for OpenAi {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
    let mut request =
      crate::openai::CreateChatCompletionRequest::try_from(&request)?;

    request.reasoning_effort.clone_from(&self.reasoning_effort);

    let mut stream = self.client.chat().create_stream(request).await?;

    let mut tool_calls = ToolCallStream::<u32>::default();

    while let Some(response) = stream.next().await {
      for choice in response?.choices {
        match choice.delta.content {
          Some(content) if !content.is_empty() => sink.delta(content)?,
          _ => {}
        }

        for chunk in choice.delta.tool_calls.unwrap_or_default() {
          for tool_call in tool_calls.push_event(chunk)? {
            sink.tool_call(tool_call)?;
          }
        }
      }
    }

    for tool_call in tool_calls.finish_all()? {
      sink.tool_call(tool_call)?;
    }

    Ok(())
  }
}
