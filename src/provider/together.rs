use {super::*, ::rig::providers::together};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("TOGETHER_API_KEY").unwrap_or_default();

  let client = together::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
