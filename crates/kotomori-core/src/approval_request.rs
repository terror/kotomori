use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ApprovalRequest {
  pub id: ApprovalId,
  pub invocation: ToolRequest,
}
