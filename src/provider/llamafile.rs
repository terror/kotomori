use {
  super::*,
  ::rig::providers::openai::{
    OpenAIConfig,
    wire::{Auth, Dialect},
  },
};

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let base_url = env::var("LLAMAFILE_API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8080".into());

  let client = OpenAIConfig::with_key(
    &Dialect::gateway("llamafile", "http://localhost:8080", ""),
    "",
  )
  .with_auth(Auth::OptionalBearer)
  .with_base_url(format!("{}/v1", base_url.trim_end_matches('/')))
  .client();

  Rig::build(client.chat(&model.name))
}
