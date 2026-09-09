use {super::*, ::rig::providers::perplexity};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("PERPLEXITY_API_KEY").unwrap_or_default();

  let client = perplexity::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
