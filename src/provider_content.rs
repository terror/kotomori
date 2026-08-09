use super::*;

#[derive(Debug)]
pub(crate) enum ProviderContent {
  Reasoning(String),
  Text(String),
  ToolCall(RawToolCall),
}
