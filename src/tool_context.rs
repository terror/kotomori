use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ToolContext {
  pub(crate) command_executor: CommandExecutor,
}
