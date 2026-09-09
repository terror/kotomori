use {super::*, ::rig::providers::groq};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("GROQ_API_KEY").unwrap_or_default();

  let client = groq::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
