use super::*;

pub struct ToolCall {
  pub approval: ApprovalPolicy,
  execute:
    Box<dyn FnOnce(ToolContext) -> BoxFuture<'static, ToolResult> + Send>,
}

impl ToolCall {
  pub fn execute(self, context: ToolContext) -> BoxFuture<'static, ToolResult> {
    (self.execute)(context)
  }

  pub fn new<F, Fut>(approval: ApprovalPolicy, execute: F) -> Self
  where
    F: FnOnce(ToolContext) -> Fut + Send + 'static,
    Fut: Future<Output = ToolResult> + Send + 'static,
  {
    Self {
      approval,
      execute: Box::new(move |context| Box::pin(execute(context))),
    }
  }
}
