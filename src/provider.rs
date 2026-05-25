use super::*;

mod anthropic;
mod fake;
mod ollama;
mod openai;

pub(crate) use {
  anthropic::Anthropic, fake::Fake, ollama::Ollama, openai::OpenAi,
};

#[async_trait]
pub(crate) trait Provider: fmt::Debug + Send + Sync {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result;
}
