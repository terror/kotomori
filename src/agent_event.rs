use super::*;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum AgentEvent {
  Delta(String),
  Done,
  Error(String),
  Message(Message),
  ReasoningDelta(String),
  ToolApprovalRequest(ApprovalRequest),
}
