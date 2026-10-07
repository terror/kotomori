use {
  super::*,
  ::rig::providers::chatgpt::{
    self,
    auth::{AuthSource, Authenticator, DeviceCodeHandler},
  },
};

#[derive(Debug)]
struct ChatGpt {
  authenticator: Authenticator,
}

impl Provider for ChatGpt {
  fn stream<'a>(
    &'a self,
    request: Request,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<AgentMessage>> {
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

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  match chatgpt::from_env() {
    Ok(client) => return Ok(Rig::build(client.completion(&model.name))),
    Err(EnvError::Variable {
      name: "CHATGPT_ACCESS_TOKEN",
      source: env::VarError::NotPresent,
    }) => {}
    Err(error) => return Err(error.into()),
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
      AuthSource::OAuth,
      config_dir.map(|path| path.join("chatgpt/auth.json")),
      DeviceCodeHandler::default(),
      true,
    ),
  }))
}
