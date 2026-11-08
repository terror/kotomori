use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
  ApprovalRequested(ApprovalRequest),
  ApprovalResolved {
    tool_id: ToolId,
    approval: ToolApproval,
  },
  MessageCommitted {
    message_id: MessageId,
    message: Message,
  },
  MessagePreview {
    message_id: MessageId,
    update: MessageUpdate,
  },
  RunCompleted,
  RunFailed(String),
  RunStarted,
  ToolCompleted {
    tool_id: ToolId,
    message_id: MessageId,
  },
  ToolDenied {
    tool_id: ToolId,
    message_id: MessageId,
  },
  ToolRequested {
    tool_id: ToolId,
    message_id: MessageId,
    invocation: ToolInvocation,
  },
  ToolStarted {
    tool_id: ToolId,
  },
}
