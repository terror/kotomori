use {super::*, ::rig::providers::mistral};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("MISTRAL_API_KEY").unwrap_or_default();

  let client = mistral::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
