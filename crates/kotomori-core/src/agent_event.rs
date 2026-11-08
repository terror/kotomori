use super::*;

#[derive(Debug, PartialEq)]
pub enum AgentEvent {
  Done,
  Error(String),
  Message(Message),
  ToolApprovalRequest(ApprovalRequest),
  Update(MessageUpdate),
  Usage(Usage),
}
