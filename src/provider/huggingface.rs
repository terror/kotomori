use {super::*, ::rig::providers::huggingface};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("HUGGINGFACE_API_KEY").unwrap_or_default();

  let client = huggingface::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
