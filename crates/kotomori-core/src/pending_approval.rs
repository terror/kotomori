use super::*;

#[derive(Debug)]
pub(super) struct PendingApproval {
  pub(super) id: ApprovalId,
  pub(super) response_sender: oneshot::Sender<ToolApproval>,
}
