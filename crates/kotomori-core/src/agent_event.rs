use super::*;

#[derive(Debug, PartialEq)]
pub enum AgentEvent {
  Message(Message),
  ToolApprovalRequest(ApprovalRequest),
  Update(MessageUpdate),
  Usage(Usage),
}
