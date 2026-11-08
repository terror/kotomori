use super::*;

pub(crate) struct ToolCall {
  pub(crate) approval: ApprovalPolicy,
  pub(crate) future: BoxFuture<'static, ToolResult>,
}
