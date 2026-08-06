use {
  super::{rig::Rig, *},
  ::rig::{
    client::{CompletionClient, ProviderClient},
    providers::azure,
  },
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let client = azure::Client::from_env()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "azure",
  }))
}
