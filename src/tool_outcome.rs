use super::*;

#[derive(
  Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolOutcome {
  #[default]
  Failure,
  Success,
}
