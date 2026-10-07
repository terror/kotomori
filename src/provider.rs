use {
  super::*,
  ::rig::{
    client::env::EnvError,
    providers::{
      cohere::Cohere,
      ollama::Ollama,
      openai::{OpenAIConfig, wire::Dialect},
      registry::ProviderRef,
    },
  },
  rig::Rig,
};

mod chatgpt;
mod copilot;
mod mock;
mod rig;

pub(crate) trait Provider: fmt::Debug + Send + Sync {
  fn stream<'a>(
    &'a self,
    request: Request,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<AgentMessage>>;
}

impl TryFrom<Model> for Arc<dyn Provider> {
  type Error = Error;

  fn try_from(model: Model) -> Result<Self> {
    let provider = match model.provider.as_str() {
      "azure" => "azure.openai/openai",
      "gemini" => "gcp.gemini/gemini",
      "llamafile" => "llamacpp/openai",
      "minimax" => "minimax/openai",
      "moonshot" => "moonshot/openai",
      "xiaomimimo" => "xiaomimimo/openai",
      "zai" => "zai/openai",
      provider => provider,
    };

    match provider {
      "chatgpt" | "chatgpt/openai" => chatgpt::build(&model),
      "cohere" => Ok(Rig::build(Cohere::from_env()?.completion(&model.name))),
      "copilot" | "copilot/openai" => copilot::build(&model),
      "galadriel" => Ok(Rig::build(
        OpenAIConfig::from_env_with(&Dialect::gateway(
          "galadriel",
          "https://api.galadriel.com/v1/verified",
          "GALADRIEL_API_KEY",
        ))?
        .client()
        .chat(&model.name),
      )),
      "mock" => Ok(Arc::new(mock::Mock)),
      "ollama" => Ok(Rig::build(Ollama::from_env()?.completion(&model.name))),
      "openai" | "openai/openai" => Ok(Rig::build(
        OpenAIConfig::from_env()?.client().chat(&model.name),
      )),
      provider => Ok(Rig::build(
        ProviderRef::parse(&format!("{provider}:{}", model.name))?
          .completion_model()?,
      )),
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
      "no registered provider is named `foo`",
    );
  }
}
