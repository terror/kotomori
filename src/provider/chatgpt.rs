use {
  super::*,
  rig::{
    client::env::EnvError,
    providers::chatgpt::{
      self,
      auth::{AuthSource, Authenticator, DeviceCodeHandler},
    },
  },
};

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

  Ok(Arc::new(ChatGpt::new(Authenticator::new(
    AuthSource::OAuth,
    config_dir.map(|path| path.join("chatgpt/auth.json")),
    DeviceCodeHandler::default(),
    true,
  ))))
}
