use {super::*, ::rig::providers::gemini};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();

  let client = gemini::Client::builder().api_key(api_key).build()?;

  Ok(Rig::build(&client, model))
}
