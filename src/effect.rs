use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
  InterruptAgent,
  RunAgent { messages: Vec<Message> },
}
