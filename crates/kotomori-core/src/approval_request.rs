use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ApprovalRequest {
  pub invocation: ToolInvocation,
  pub run_id: RunId,
  pub tool_id: ToolId,
}
