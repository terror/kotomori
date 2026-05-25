use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderEvent {
  Delta(String),
  ToolCall(ToolCall),
}
