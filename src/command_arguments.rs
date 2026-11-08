use super::*;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandArguments {
  pub(crate) command: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
}
