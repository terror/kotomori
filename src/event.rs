use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
  Action(Action),
  AgentDelta(String),
  AgentDone,
  Error(String),
}
