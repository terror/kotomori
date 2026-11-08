use super::*;

#[derive(Clone, Debug)]
pub(crate) struct AgentConfig {
  pub(crate) model: Model,
  pub(crate) tool_executor: ToolExecutor,
  pub(crate) yolo: bool,
}
