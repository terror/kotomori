use {
  super::*,
  ::rig::providers::copilot::{Copilot, auth::Authenticator},
};

#[derive(Debug)]
pub struct GithubCopilot {
  authenticator: Authenticator,
}

impl GithubCopilot {
  #[must_use]
  pub fn new(authenticator: Authenticator) -> Self {
    Self { authenticator }
  }
}

impl Provider for GithubCopilot {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<ProviderResponse>> {
    Box::pin(async move {
      let client = Copilot::new("").authenticate(&self.authenticator).await?;

      Rig::build(client.completion(&request.model.name))
        .stream(request, sink)
        .await
    })
  }
}
