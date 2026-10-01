#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum AgentActivity {
  Reasoning,
  Streaming,
  #[default]
  Waiting,
}
