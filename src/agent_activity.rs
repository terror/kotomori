#[derive(Debug, Default)]
pub(crate) enum AgentActivity {
  #[default]
  Idle,
  Streaming {
    content: String,
    reasoning: String,
  },
  Waiting,
}
