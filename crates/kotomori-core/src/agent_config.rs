use super::*;

#[derive(Clone, Debug)]
pub struct AgentConfig {
  pub model: Model,
  pub tools: ToolRegistry,
  pub yolo: bool,
}
