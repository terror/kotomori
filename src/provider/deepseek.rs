use {super::*, ::rig::providers::deepseek};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("DEEPSEEK_API_KEY").unwrap_or_default();

  let client = deepseek::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
