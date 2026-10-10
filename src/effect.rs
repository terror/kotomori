use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Effect {
  CopyToClipboard(String),
  InterruptAgent,
  RunAgent { messages: Vec<Message>, run_id: u64 },
  SaveSession,
}
