use {super::*, ::rig::providers::openai};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("LLAMACPP_API_KEY").unwrap_or_else(|_| "none".into());

  let base_url = env::var("LLAMACPP_API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8080/v1".into());

  let client = openai::OpenAIConfig::new(api_key)
    .with_base_url(base_url)
    .client();

  Rig::build(client.chat(&model.name))
}
