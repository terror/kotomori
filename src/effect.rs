use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Effect {
  Compact { messages: Vec<Message>, run_id: u64 },
  InterruptAgent,
  RunAgent { messages: Vec<Message>, run_id: u64 },
}
