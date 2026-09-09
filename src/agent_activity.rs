#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum AgentActivity {
  #[default]
  Idle,
  Reasoning(String),
  Streaming(String),
  Waiting,
}
