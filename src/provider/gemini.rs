use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::gemini},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();

  let client = gemini::Client::builder().api_key(api_key).build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "gemini",
  }))
}
