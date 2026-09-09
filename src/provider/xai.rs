use {super::*, ::rig::providers::xai};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("XAI_API_KEY").unwrap_or_default();

  let client = xai::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
