use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::openai},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("LLAMACPP_API_KEY").unwrap_or_else(|_| "none".into());

  let base_url = env::var("LLAMACPP_API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8080/v1".into());

  let client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .base_url(base_url)
    .build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "llamacpp",
  }))
}
