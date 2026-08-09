use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Effect {
  InterruptAgent,
  RunAgent { messages: Vec<Message>, run_id: u64 },
}
