use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Effect {
  CopyToClipboard(String),
  ForkSession,
  InterruptAgent,
  RunAgent {
    conversation: Conversation,
    run_id: u64,
  },
  SaveSession,
}
