use super::*;

mod chatgpt;
mod copilot;
mod mock;

pub(crate) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
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
    "chatgpt" | "chatgpt/openai" => chatgpt::build(model),
    "cohere" => Rig::cohere(&model.name),
    "copilot" | "copilot/openai" => copilot::build(model),
    "galadriel" => Rig::gateway(
      &model.name,
      "galadriel",
      "https://api.galadriel.com/v1/verified",
      "GALADRIEL_API_KEY",
    ),
    "mock" => Ok(Arc::new(mock::Mock)),
    "ollama" => Rig::ollama(&model.name),
    "openai" | "openai/openai" => Rig::openai(&model.name),
    provider => Rig::provider(provider, &model.name),
  }
}
