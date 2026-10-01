use {super::*, ::rig::providers::huggingface};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("HUGGINGFACE_API_KEY").unwrap_or_default();

  let client = huggingface::new(api_key);

  Rig::build(client.completion(&model.name))
}
