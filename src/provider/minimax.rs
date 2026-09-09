use {super::*, ::rig::providers::minimax};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let api_key = env::var("MINIMAX_API_KEY").unwrap_or_default();

  let mut builder = minimax::Client::builder().api_key(api_key);

  if let Ok(base_url) = env::var("MINIMAX_API_BASE") {
    builder = builder.base_url(base_url);
  }

  let client = builder.build()?;

  Ok(Rig::build(&client, model))
}
