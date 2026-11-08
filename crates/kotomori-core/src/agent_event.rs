use super::*;

#[derive(Debug, PartialEq)]
pub enum AgentEvent {
  Message(Message),
  ToolApprovalRequest(ApprovalRequest),
  ToolStarted(CallId),
  Update(MessageUpdate),
  Usage(Usage),
}
