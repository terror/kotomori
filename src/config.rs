use super::*;

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
  pub(crate) default_model: Option<String>,
  pub(crate) default_provider: Option<String>,
}

impl Config {
  const APP_NAME: &'static str = "kotomori";
  const CONFIG_NAME: &'static str = "config";

  pub(crate) fn load() -> Result<Self> {
    match env::var_os("KOTOMORI_CONFIG") {
      Some(path) => confy::load_path(PathBuf::from(path)),
      None => confy::load(Self::APP_NAME, Some(Self::CONFIG_NAME)),
    }
    .context("failed to load configuration")
  }
}
