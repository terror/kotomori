use {super::*, ::rig::providers::together};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("TOGETHER_API_KEY").unwrap_or_default();

  let client = together::new(api_key);

  Rig::build(client.completion(&model.name))
}
