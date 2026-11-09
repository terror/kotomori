use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResponseResult {
  pub finish_reason: Option<FinishReason>,
  pub id: ResponseId,
  pub outcome: ResponseOutcome,
  pub usage: Usage,
}
