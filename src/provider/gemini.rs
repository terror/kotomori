use {super::*, ::rig::providers::gemini};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();

  let client = gemini::Gemini::new(api_key);

  Rig::build(client.completion(&model.name))
}
