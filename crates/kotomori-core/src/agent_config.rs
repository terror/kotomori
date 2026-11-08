use super::*;

#[derive(Clone, Debug)]
pub struct AgentConfig {
  pub max_rounds: usize,
  pub model: Model,
  pub tools: ToolRegistry,
  pub yolo: bool,
}

impl AgentConfig {
  pub const DEFAULT_MAX_ROUNDS: usize = 100;
}
