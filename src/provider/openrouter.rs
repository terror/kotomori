use {super::*, ::rig::providers::openrouter};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_default();

  let client = openrouter::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
