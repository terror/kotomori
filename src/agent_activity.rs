#[derive(Debug, Default)]
pub(crate) enum AgentActivity {
  #[default]
  Idle,
  Reasoning(String),
  Streaming(String),
  Waiting,
}
