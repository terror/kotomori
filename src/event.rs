use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
  Action(Action),
  AgentDelta(String),
  AgentDone,
  AgentToolCall(ToolInvocation),
  AgentToolResult { id: String, result: ToolResult },
  Error(String),
  Tick(Duration),
  ToolApprovalRequest(ApprovalRequest),
}
