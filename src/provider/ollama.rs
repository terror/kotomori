use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::ollama},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("OLLAMA_API_KEY").unwrap_or_default();

  let base_url = env::var("OLLAMA_API_BASE_URL")
    .or_else(|_| env::var("OLLAMA_HOST"))
    .unwrap_or_else(|_| "http://localhost:11434".into())
    .trim_end_matches('/')
    .to_string();

  let client = ollama::Client::builder()
    .api_key(api_key)
    .base_url(base_url)
    .build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "ollama",
  }))
}
