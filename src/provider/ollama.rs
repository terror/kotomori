use super::*;

#[derive(Debug)]
pub(crate) struct Ollama {
  openai: OpenAi,
}

impl Ollama {
  pub(crate) fn new() -> Self {
    Self {
      openai: OpenAi::with_config(
        crate::openai::OpenAIConfig::new()
          .with_api_base(format!(
            "{}/v1",
            env::var("OLLAMA_HOST")
              .unwrap_or_else(|_| "http://localhost:11434".into())
          ))
          .with_api_key("ollama"),
      )
      .with_reasoning_effort(crate::openai::ReasoningEffort::None),
    }
  }
}

#[async_trait]
impl Provider for Ollama {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
    self.openai.stream(request, sink).await
  }
}
