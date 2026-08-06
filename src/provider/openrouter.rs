use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::openrouter},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_default();

  let client = openrouter::Client::builder().api_key(api_key).build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "openrouter",
  }))
}
