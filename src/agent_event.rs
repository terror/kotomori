use super::*;

#[derive(Debug, PartialEq)]
pub(crate) enum AgentEvent {
  Done,
  Error(String),
  Message(Message),
  ToolApprovalRequest(ApprovalRequest),
  Update(MessageUpdate),
}
