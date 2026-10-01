use {super::*, ::rig::providers::xai};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("XAI_API_KEY").unwrap_or_default();

  let client = xai::new(api_key);

  Rig::build(client.completion(&model.name))
}
