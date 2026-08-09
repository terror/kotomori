use super::*;

#[async_trait]
pub(crate) trait ToolCall:
  DeserializeOwned + Display + JsonSchema + Send + Sync
{
  const DESCRIPTION: &'static str;
  const NAME: &'static str;

  fn action(tense: ToolActionTense) -> &'static str;

  fn approval(&self) -> ApprovalPolicy {
    ApprovalPolicy::None
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    Vec::new()
  }

  async fn execute(&self, context: &ToolContext) -> ToolResult;
}
