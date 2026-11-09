use {
  super::*,
  ::rig::providers::chatgpt::{self, auth::Authenticator},
};

#[derive(Debug)]
pub struct ChatGpt {
  authenticator: Authenticator,
}

impl ChatGpt {
  #[must_use]
  pub fn new(authenticator: Authenticator) -> Self {
    Self { authenticator }
  }
}

impl Provider for ChatGpt {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<ProviderResponse>> {
    Box::pin(async move {
      let client = OpenAIConfig::with_key(&chatgpt::DIALECT, "")
        .client()
        .authenticate(&self.authenticator)
        .await?;

      Rig::build(client.completion(&request.model.name))
        .stream(request, sink)
        .await
    })
  }
}
