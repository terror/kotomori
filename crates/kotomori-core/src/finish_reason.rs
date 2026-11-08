use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
  ContentFilter,
  Length,
  Other(String),
  Stop,
  ToolCalls,
}
