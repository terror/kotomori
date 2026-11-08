use super::*;

#[derive(Clone, Debug)]
pub(crate) struct AgentConfig {
  pub(crate) model: Model,
  pub(crate) tools: ToolRegistry,
  pub(crate) yolo: bool,
}
