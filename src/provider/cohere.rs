use {super::*, ::rig::providers::cohere};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("COHERE_API_KEY").unwrap_or_default();

  let client = cohere::Cohere::new(api_key);

  Rig::build(client.completion(&model.name))
}
