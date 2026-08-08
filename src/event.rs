use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
  Action(Action),
  Agent { event: AgentEvent, run_id: u64 },
  Error(String),
  Tick(Duration),
}
