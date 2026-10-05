use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ToolContext {
  pub(crate) command_executor: CommandExecutor,
  pub(crate) directory: PathBuf,
}
