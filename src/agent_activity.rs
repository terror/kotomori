#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum AgentActivity {
  Reasoning(String),
  Streaming(String),
  #[default]
  Waiting,
}
