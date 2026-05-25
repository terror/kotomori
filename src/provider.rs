use super::*;

mod fake;
mod ollama;

pub(crate) use {fake::Fake, ollama::Ollama};

#[async_trait]
pub(crate) trait Provider: fmt::Debug + Send + Sync {
  async fn stream(
    &self,
    request: CompletionRequest,
    sink: ProviderSink,
  ) -> Result;
}
