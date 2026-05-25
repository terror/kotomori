use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
  RunAgent { messages: Vec<Message> },
}
