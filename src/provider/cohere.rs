use {super::*, ::rig::providers::cohere};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("COHERE_API_KEY").unwrap_or_default();

  let client = cohere::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
