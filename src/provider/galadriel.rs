use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::galadriel},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("GALADRIEL_API_KEY").unwrap_or_default();

  let mut builder = galadriel::Client::builder().api_key(api_key);

  if let Ok(fine_tune_api_key) = env::var("GALADRIEL_FINE_TUNE_API_KEY") {
    builder = builder.fine_tune_api_key(fine_tune_api_key);
  }

  let client = builder.build()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "galadriel",
  }))
}
