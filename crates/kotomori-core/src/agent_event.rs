use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEvent {
  Message {
    message: Message,
    response: Option<ResponseId>,
    run: RunId,
  },
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
  Update {
    response: ResponseId,
    update: MessageUpdate,
  },
}
