use {super::*, ::rig::providers::openai};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("LLAMACPP_API_KEY").unwrap_or_else(|_| "none".into());

  let base_url = env::var("LLAMACPP_API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8080/v1".into());

  let client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .base_url(base_url)
    .build()?;

  Ok(Rig::build(&client, model))
}
