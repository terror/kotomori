use {super::*, ::rig::providers::perplexity};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("PERPLEXITY_API_KEY").unwrap_or_default();

  let client = perplexity::new(api_key);

  Rig::build(client.completion(&model.name))
}
