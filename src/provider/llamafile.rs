use {super::*, ::rig::providers::llamafile};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let base_url = env::var("LLAMAFILE_API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8080".into());

  let client = llamafile::Client::from_url(&base_url)?;

  Ok(Rig::build(&client, model))
}
