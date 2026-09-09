use {super::*, ::rig::providers::openai};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();

  let mut builder = openai::CompletionsClient::builder().api_key(api_key);

  if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
    builder = builder.base_url(base_url);
  }

  let client = builder.build()?;

  Ok(Rig::build(&client, model))
}
