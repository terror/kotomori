use {
  super::{rig::Rig, *},
  ::rig::{client::CompletionClient, providers::llamafile},
};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let base_url = env::var("LLAMAFILE_API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8080".into());

  let client = llamafile::Client::from_url(&base_url)?;

  let completion_model =
    CompletionClient::completion_model(&client, &model.name);

  Ok(Arc::new(Rig {
    model: completion_model,
    provider: "llamafile",
  }))
}
