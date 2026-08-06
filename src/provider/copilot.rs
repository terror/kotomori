use {
  super::{rig::Rig, *},
  ::rig::{
    client::{CompletionClient, ProviderClient},
    providers::copilot,
  },
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let client = copilot::Client::from_env()?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "copilot",
  }))
}
