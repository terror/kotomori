use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderOutput {
  pub(crate) content: Vec<AgentMessageContent>,
}

impl ProviderOutput {
  pub(crate) fn tool_calls(&self) -> impl Iterator<Item = &ToolInvocation> {
    self.content.iter().filter_map(|content| match content {
      AgentMessageContent::Reasoning(_) => None,
      AgentMessageContent::Text(_) => None,
      AgentMessageContent::ToolCall(invocation) => Some(invocation),
    })
  }
}
