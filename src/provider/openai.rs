use {super::*, ::rig::providers::openai};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();

  let config = openai::OpenAIConfig::new(api_key);

  let config = if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
    config.with_base_url(base_url)
  } else {
    config
  };

  let client = config.client();

  Rig::build(client.chat(&model.name))
}
