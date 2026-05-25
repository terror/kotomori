use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
  Action(Action),
  AgentDelta(String),
  AgentDone,
  AgentToolCall(RawToolCall),
  Error(String),
  Tick,
}
