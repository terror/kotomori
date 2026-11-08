use super::*;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandArguments {
  pub command: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cwd: Option<PathBuf>,
}
