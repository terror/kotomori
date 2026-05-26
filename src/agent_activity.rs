#[derive(Debug, Default)]
pub(crate) enum AgentActivity {
  #[default]
  Idle,
  Streaming(String),
  Waiting,
}
