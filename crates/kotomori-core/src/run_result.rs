use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct RunResult {
  pub conversation: Conversation,
  pub entries: Vec<RunEntry>,
  pub outcome: RunOutcome,
  pub responses: Vec<ResponseResult>,
  pub usage: Usage,
}
