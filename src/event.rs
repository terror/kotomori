use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
  Action(Action),
  AgentDone,
  Error(String),
  Provider(ProviderEvent),
  Tick,
}
