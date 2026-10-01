use {super::*, ::rig::providers::groq};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("GROQ_API_KEY").unwrap_or_default();

  let client = groq::new(api_key);

  Rig::build(client.completion(&model.name))
}
