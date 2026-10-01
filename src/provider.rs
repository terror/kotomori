use {super::*, rig::Rig};

mod anthropic;
mod azure;
mod chatgpt;
mod cohere;
mod copilot;
mod deepseek;
mod galadriel;
mod gemini;
mod groq;
mod huggingface;
mod llamacpp;
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
  #[allow(clippy::double_must_use)]
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result;
}

impl TryFrom<Model> for Arc<dyn Provider> {
  type Error = Error;

  fn try_from(model: Model) -> Result<Self> {
    match model.provider.as_str() {
      "anthropic" => Ok(anthropic::build(&model)),
      "azure" => azure::build(&model),
      "chatgpt" => chatgpt::build(&model),
      "cohere" => Ok(cohere::build(&model)),
      "copilot" => Ok(copilot::build(&model)),
      "deepseek" => Ok(deepseek::build(&model)),
      "galadriel" => Ok(galadriel::build(&model)),
      "gemini" => Ok(gemini::build(&model)),
      "groq" => Ok(groq::build(&model)),
      "huggingface" => Ok(huggingface::build(&model)),
      "llamacpp" => Ok(llamacpp::build(&model)),
      "llamafile" => Ok(llamafile::build(&model)),
      "minimax" => Ok(minimax::build(&model)),
      "mistral" => Ok(mistral::build(&model)),
      "mock" => Ok(Arc::new(mock::Mock)),
      "moonshot" => Ok(moonshot::build(&model)),
      "ollama" => Ok(ollama::build(&model)),
      "openai" => Ok(openai::build(&model)),
      "openrouter" => Ok(openrouter::build(&model)),
      "perplexity" => Ok(perplexity::build(&model)),
      "together" => Ok(together::build(&model)),
      "xai" => Ok(xai::build(&model)),
      "xiaomimimo" => Ok(xiaomimimo::build(&model)),
      "zai" => Ok(zai::build(&model)),
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
