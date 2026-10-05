use super::*;

#[derive(Debug, PartialEq)]
pub(crate) enum Event {
  Action(Action),
  Agent { event: AgentEvent, run_id: u64 },
  Error(String),
  SessionSaved(Result<SavedSession, String>),
  Tick(Duration),
}
