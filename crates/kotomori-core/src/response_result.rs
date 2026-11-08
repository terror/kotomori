use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ResponseResult {
  pub finish_reason: Option<FinishReason>,
  pub id: ResponseId,
  pub outcome: ResponseOutcome,
  pub usage: Usage,
}
