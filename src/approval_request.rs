use super::*;

pub(crate) struct ApprovalRequest {
  pub(crate) invocation: ToolInvocation,
  response_sender: oneshot::Sender<ToolApproval>,
}

impl ApprovalRequest {
  pub(crate) fn new(
    invocation: ToolInvocation,
  ) -> (Self, oneshot::Receiver<ToolApproval>) {
    let (response_sender, response_receiver) = oneshot::channel();

    (
      Self {
        invocation,
        response_sender,
      },
      response_receiver,
    )
  }

  pub(crate) fn respond(self, approval: ToolApproval) {
    let _ = self.response_sender.send(approval);
  }
}

impl Debug for ApprovalRequest {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("ApprovalRequest")
      .field("invocation", &self.invocation)
      .finish_non_exhaustive()
  }
}

impl Eq for ApprovalRequest {}

impl PartialEq for ApprovalRequest {
  fn eq(&self, other: &Self) -> bool {
    self.invocation == other.invocation
  }
}
