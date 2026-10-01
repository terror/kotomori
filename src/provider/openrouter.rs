use {super::*, ::rig::providers::openrouter};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_default();

  let client = openrouter::new(api_key);

  Rig::build(client.completion(&model.name))
}
