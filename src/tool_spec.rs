use super::*;

#[async_trait]
pub(crate) trait ToolSpec:
  DeserializeOwned + Into<ToolInvocationKind> + JsonSchema + Send + Sync
{
  const DESCRIPTION: &'static str;

  const NAME: &'static str;

  fn action(tense: ToolActionTense) -> &'static str;

  fn details(&self) -> Vec<(&'static str, String)> {
    Vec::new()
  }

  fn display(&self) -> String;

  async fn execute(&self, executor: &Executor) -> ToolResult;

  fn requires_approval(&self) -> bool {
    false
  }

  fn subject(&self) -> String;
}
