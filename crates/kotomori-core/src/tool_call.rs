use super::*;

pub struct ToolCall {
  pub approval: ApprovalPolicy,
  pub future: BoxFuture<'static, ToolResult>,
}
