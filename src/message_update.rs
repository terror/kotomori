use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MessageUpdate {
  MessageId(String),
  Reasoning(Reasoning),
  ReasoningAppend(Reasoning),
  ReasoningDelta { delta: String, id: Option<String> },
  Text(String),
  ToolCall(::rig::message::ToolCall),
}
