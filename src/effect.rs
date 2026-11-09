use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Effect {
  CopyToClipboard(String),
  ForkSession,
  InterruptAgent,
  ResolveApproval {
    approval: ToolApproval,
    id: ApprovalId,
  },
  RunAgent {
    run_id: u64,
  },
  SaveSession,
}
