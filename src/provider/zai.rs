use {
  super::*,
  ::rig::providers::openai::{OpenAIConfig, wire::ZAI},
};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("ZAI_API_KEY").unwrap_or_default();

  let config = OpenAIConfig::with_key(&ZAI, api_key);

  let config = if let Ok(base_url) = env::var("ZAI_API_BASE") {
    config.with_base_url(base_url)
  } else {
    config
  };

  Rig::build(config.client().completion(&model.name))
}
