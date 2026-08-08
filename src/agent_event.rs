use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentEvent {
  Delta(String),
  Done,
  Error(String),
  ReasoningDelta(String),
  ToolApprovalRequest(ApprovalRequest),
  ToolCall(ToolInvocation),
  ToolResult { id: String, result: ToolResult },
}
