use {super::*, ::rig::providers::mistral};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("MISTRAL_API_KEY").unwrap_or_default();

  let client = mistral::new(api_key);

  Rig::build(client.completion(&model.name))
}
