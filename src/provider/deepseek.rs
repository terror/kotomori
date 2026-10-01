use {super::*, ::rig::providers::deepseek};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("DEEPSEEK_API_KEY").unwrap_or_default();

  let client = deepseek::new(api_key);

  Rig::build(client.completion(&model.name))
}
