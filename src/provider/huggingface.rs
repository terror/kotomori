use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::huggingface},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("HUGGINGFACE_API_KEY").unwrap_or_default();

  let client = huggingface::Client::builder().api_key(api_key).build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "huggingface",
  }))
}
