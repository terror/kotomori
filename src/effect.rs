use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Effect {
  CopyToClipboard(String),
  ForkSession,
  InterruptAgent,
  RespondToApproval(ApprovalResponse),
  RunAgent { conversation: Conversation },
  SaveSession,
}
