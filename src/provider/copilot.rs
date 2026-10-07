use {
  super::*,
  ::rig::providers::copilot::{
    Copilot,
    auth::{self, AuthSource, Authenticator, DeviceCodeHandler},
  },
};

#[derive(Debug)]
struct GithubCopilot {
  authenticator: Authenticator,
}

impl Provider for GithubCopilot {
  fn stream<'a>(
    &'a self,
    request: Request,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<AgentMessage>> {
    Box::pin(async move {
      let client = Copilot::new("").authenticate(&self.authenticator).await?;

      Rig::build(client.completion(&request.model.name))
        .stream(request, sink)
        .await
    })
  }
}

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  match Copilot::from_env() {
    Ok(client) => return Ok(Rig::build(client.completion(&model.name))),
    Err(EnvError::Variable {
      name: "GITHUB_COPILOT_API_KEY",
      source: env::VarError::NotPresent,
    }) => {}
    Err(error) => return Err(error.into()),
  }

  let source = ["COPILOT_GITHUB_ACCESS_TOKEN", "GITHUB_TOKEN"]
    .iter()
    .find_map(|name| {
      env::var(name).ok().filter(|value| !value.trim().is_empty())
    })
    .map_or(AuthSource::OAuth, AuthSource::GitHubAccessToken);

  let token_dir = auth::default_token_dir();

  Ok(Arc::new(GithubCopilot {
    authenticator: Authenticator::new(
      source,
      token_dir.as_ref().map(|path| path.join("access-token")),
      token_dir.map(|path| path.join("api-key.json")),
      DeviceCodeHandler::default(),
      true,
    ),
  }))
}
