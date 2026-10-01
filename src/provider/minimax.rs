use {
  super::*,
  ::rig::providers::openai::{OpenAIConfig, wire::MINIMAX},
};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("MINIMAX_API_KEY").unwrap_or_default();

  let config = OpenAIConfig::with_key(&MINIMAX, api_key);

  let config = if let Ok(base_url) = env::var("MINIMAX_API_BASE") {
    config.with_base_url(base_url)
  } else {
    config
  };

  Rig::build(config.client().completion(&model.name))
}
