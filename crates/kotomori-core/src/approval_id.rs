use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalId {
  pub call: CallId,
  pub response: ResponseId,
}
