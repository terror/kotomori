use super::*;

mod anthropic;
mod azure;
mod chatgpt;
mod cohere;
mod deepseek;
mod gemini;
mod groq;
mod huggingface;
mod llamafile;
mod minimax;
mod mistral;
mod mock;
mod moonshot;
mod ollama;
mod openai;
mod openrouter;
mod perplexity;
mod rig;
mod together;
mod xai;
mod xiaomimimo;
mod zai;

#[async_trait]
pub(crate) trait Provider: fmt::Debug + Send + Sync {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result;
}

impl TryFrom<Model> for Arc<dyn Provider> {
  type Error = Error;

  fn try_from(model: Model) -> Result<Self> {
    match model.provider.as_str() {
      "anthropic" => anthropic::build(&model),
      "azure" => azure::build(&model),
      "chatgpt" => chatgpt::build(&model),
      "cohere" => cohere::build(&model),
      "deepseek" => deepseek::build(&model),
      "gemini" => gemini::build(&model),
      "groq" => groq::build(&model),
      "huggingface" => huggingface::build(&model),
      "llamafile" => llamafile::build(&model),
      "minimax" => minimax::build(&model),
      "mistral" => mistral::build(&model),
      "mock" => Ok(Arc::new(mock::Mock)),
      "moonshot" => moonshot::build(&model),
      "ollama" => ollama::build(&model),
      "openai" => openai::build(&model),
      "openrouter" => openrouter::build(&model),
      "perplexity" => perplexity::build(&model),
      "together" => together::build(&model),
      "xai" => xai::build(&model),
      "xiaomimimo" => xiaomimimo::build(&model),
      "zai" => zai::build(&model),
      provider => bail!("unknown provider `{provider}`"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_unknown_provider() {
    assert_eq!(
      Arc::<dyn Provider>::try_from(Model {
        name: "bar".into(),
        provider: "foo".into(),
      })
      .unwrap_err()
      .to_string(),
      "unknown provider `foo`",
    );
  }
}
