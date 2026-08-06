use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::xai},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("XAI_API_KEY").unwrap_or_default();

  let client = xai::Client::builder().api_key(api_key).build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "xai",
  }))
}
