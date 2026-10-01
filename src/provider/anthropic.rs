use {super::*, ::rig::providers::anthropic};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("ANTHROPIC_API_KEY")
    .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN"))
    .unwrap_or_default();

  let base_url = env::var("ANTHROPIC_BASE_URL")
    .unwrap_or_else(|_| "https://api.anthropic.com".into());

  let client = anthropic::AnthropicConfig::new(api_key)
    .with_base_url(base_url)
    .client();

  Rig::build(client.completion(&model.name))
}
