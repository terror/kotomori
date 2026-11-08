use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalResponse {
  pub approval: ToolApproval,
  pub run_id: RunId,
  pub tool_id: ToolId,
}
