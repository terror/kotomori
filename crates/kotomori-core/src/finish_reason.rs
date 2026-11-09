use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
  ContentFilter,
  Error(String),
  Length,
  Other(String),
  Stop,
  ToolCalls,
}

impl FinishReason {
  pub(super) fn outcome(&self) -> Option<RunOutcome> {
    match self {
      Self::ContentFilter => Some(RunOutcome::Failed(
        "completion filtered by the provider".into(),
      )),
      Self::Error(reason) => Some(RunOutcome::Failed(format!(
        "provider response ended with `{reason}`"
      ))),
      Self::Length => Some(RunOutcome::Limited(RunLimit::OutputTokens)),
      Self::Other(_) | Self::Stop | Self::ToolCalls => None,
    }
  }
}
