use {
  super::*,
  ::rig::providers::copilot::{
    Copilot, CopilotConfig,
    auth::{self, AuthSource, Authenticator, DeviceCodeHandler},
  },
};

#[derive(Debug)]
struct GithubCopilot {
  authenticator: Authenticator,
  client: Copilot,
  model: String,
}

#[async_trait]
impl Provider for GithubCopilot {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    let client = self
      .client
      .clone()
      .authenticate(&self.authenticator)
      .await?;

    Rig::build(client.completion(&self.model))
      .stream(request, sink)
      .await
  }
}

pub(super) fn build(model: &Model) -> Arc<dyn Provider> {
  let source = if let Some(api_key) =
    env_value(&["GITHUB_COPILOT_API_KEY", "COPILOT_API_KEY"])
  {
    AuthSource::ApiKey(api_key)
  } else if let Some(access_token) =
    env_value(&["COPILOT_GITHUB_ACCESS_TOKEN", "GITHUB_TOKEN"])
  {
    AuthSource::GitHubAccessToken(access_token)
  } else {
    AuthSource::OAuth
  };

  let config = CopilotConfig::new("");

  let config = if let Some(base_url) =
    env_value(&["GITHUB_COPILOT_API_BASE", "COPILOT_BASE_URL"])
  {
    config.with_base_url(base_url)
  } else {
    config
  };

  let token_dir = auth::default_token_dir();

  Arc::new(GithubCopilot {
    authenticator: Authenticator::new(
      source,
      token_dir.as_ref().map(|path| path.join("access-token")),
      token_dir.map(|path| path.join("api-key.json")),
      DeviceCodeHandler::default(),
      true,
    ),
    client: config.client(),
    model: model.name.clone(),
  })
}

fn env_value(names: &[&str]) -> Option<String> {
  names.iter().find_map(|name| {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
  })
}
