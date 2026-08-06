use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::anthropic},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("ANTHROPIC_API_KEY")
    .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN"))
    .unwrap_or_default();

  let base_url = env::var("ANTHROPIC_BASE_URL")
    .unwrap_or_else(|_| "https://api.anthropic.com".into());

  let client = anthropic::Client::builder()
    .api_key(api_key)
    .base_url(base_url)
    .build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "anthropic",
  }))
}
