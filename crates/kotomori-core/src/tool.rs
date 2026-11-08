use super::*;

pub trait Tool: Debug + Send + Sync {
  fn definition(&self) -> ToolDefinition;

  fn prepare(&self, arguments: Value) -> Result<ToolCall>;
}
