use {super::*, ::rig::providers::zai};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("ZAI_API_KEY").unwrap_or_default();

  let mut builder = zai::Client::builder().api_key(api_key);

  if let Ok(base_url) = env::var("ZAI_API_BASE") {
    builder = builder.base_url(base_url);
  }

  let client = builder.build()?;

  Ok(Rig::build(&client, model))
}
