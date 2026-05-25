use super::*;

mod fake;
mod ollama;

pub(crate) use {fake::Fake, ollama::Ollama};

pub(crate) trait Provider: fmt::Debug + Send + Sync {
  fn stream(
    &self,
    request: CompletionRequest,
    sink: ProviderSink,
  ) -> BoxFuture<'_, Result>;
}
