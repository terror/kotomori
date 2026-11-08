use super::*;

mod command;

pub(crate) use command::CommandTool;

pub(crate) trait Tool: Debug + Send + Sync {
  fn definition(&self) -> ToolDefinition;

  fn prepare(&self, arguments: Value) -> Result<ToolCall>;
}
