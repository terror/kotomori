use {
  super::*,
  ::rig::providers::openai::{OpenAIConfig, wire::Dialect},
};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let api_key = env::var("GALADRIEL_API_KEY").unwrap_or_default();

  let client = OpenAIConfig::with_key(
    &Dialect::gateway(
      "galadriel",
      "https://api.galadriel.com/v1/verified",
      "GALADRIEL_API_KEY",
    ),
    api_key,
  )
  .client();

  Rig::build(client.chat(&model.name))
}
