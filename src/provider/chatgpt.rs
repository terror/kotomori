use {
  super::*,
  ::rig::providers::{
    chatgpt::{
      self,
      auth::{AuthSource, Authenticator, DeviceCodeHandler},
    },
    openai::{OpenAI, OpenAIConfig},
  },
};

#[derive(Debug)]
struct ChatGpt {
  authenticator: Authenticator,
  client: OpenAI,
  model: String,
}

#[async_trait]
impl Provider for ChatGpt {
  async fn stream(
    &self,
    request: Request,
    sink: &ProviderSink,
  ) -> Result<AgentMessage> {
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

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let access_token = ::rig::client::env::optional("CHATGPT_ACCESS_TOKEN")?;

  let source = if let Some(access_token) = access_token {
    AuthSource::AccessToken {
      access_token,
      account_id: ::rig::client::env::optional("CHATGPT_ACCOUNT_ID")?,
    }
  } else {
    AuthSource::OAuth
  };

  let config = OpenAIConfig::with_key(&chatgpt::DIALECT, "");

  let config = if let Some(base_url) =
    ::rig::client::env::optional("CHATGPT_API_BASE")?
      .or(::rig::client::env::optional("OPENAI_CHATGPT_API_BASE")?)
  {
    config.with_base_url(base_url)
  } else {
    config
  };

  let mut config = if let Ok(instructions) =
    env::var("CHATGPT_DEFAULT_INSTRUCTIONS")
    && !instructions.trim().is_empty()
  {
    config.with_instructions(instructions)
  } else {
    config
  };

  if let Some(identity) = config.identity.as_mut() {
    if let Ok(originator) = env::var("CHATGPT_ORIGINATOR")
      && !originator.is_empty()
    {
      identity.originator = originator;
    }

    if let Ok(user_agent) = env::var("CHATGPT_USER_AGENT")
      && !user_agent.is_empty()
    {
      identity.user_agent = user_agent;
    }
  }

  let config_dir = if cfg!(target_os = "windows") {
    env::var_os("APPDATA").map(PathBuf::from)
  } else {
    env::var_os("XDG_CONFIG_HOME")
      .map(PathBuf::from)
      .or_else(|| {
        env::var_os("HOME").map(|path| PathBuf::from(path).join(".config"))
      })
  };

  Ok(Arc::new(ChatGpt {
    authenticator: Authenticator::new(
      source,
      config_dir.map(|path| path.join("chatgpt/auth.json")),
      DeviceCodeHandler::default(),
      true,
    ),
    client: config.client(),
    model: model.name.clone(),
  }))
}
