use super::*;

#[derive(Debug, PartialEq)]
pub enum AgentEvent {
  Message(Message),
  ResponseFinished(ResponseResult),
  ResponseStarted(ResponseId),
  ToolApprovalRequest(ApprovalRequest),
  ToolFinished {
    call: CallId,
    response: ResponseId,
    result: ToolResult,
  },
  ToolProgress {
    call: CallId,
    progress: Value,
    response: ResponseId,
  },
  ToolStarted {
    call: CallId,
    name: ToolName,
    response: ResponseId,
  },
  Update(MessageUpdate),
}
