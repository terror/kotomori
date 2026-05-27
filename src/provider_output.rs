use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderOutput {
  pub(crate) content: String,
  pub(crate) reasoning: String,
  pub(crate) tool_calls: Vec<ToolInvocation>,
}
