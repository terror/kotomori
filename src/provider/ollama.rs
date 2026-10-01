use {super::*, ::rig::providers::ollama};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("OLLAMA_API_KEY").unwrap_or_default();

  let base_url = env::var("OLLAMA_API_BASE_URL")
    .or_else(|_| env::var("OLLAMA_HOST"))
    .unwrap_or_else(|_| "http://localhost:11434".into())
    .trim_end_matches('/')
    .to_string();

  let client = ollama::OllamaConfig::new()
    .with_api_key(api_key)
    .with_base_url(base_url)
    .client();

  Rig::build(client.completion(&model.name))
}
