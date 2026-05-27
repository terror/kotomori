use super::*;

#[derive(Clone)]
pub(crate) struct ApprovalRequest {
  invocation: ToolInvocation,
  response_sender: Arc<Mutex<Option<oneshot::Sender<ToolApproval>>>>,
}

impl ApprovalRequest {
  pub(crate) fn approve(&self) {
    self.respond(ToolApproval::Approved);
  }

  pub(crate) fn deny(&self) {
    self.respond(ToolApproval::Denied);
  }

  pub(crate) fn invocation(&self) -> &ToolInvocation {
    &self.invocation
  }

  pub(crate) fn new(
    invocation: ToolInvocation,
  ) -> (Self, oneshot::Receiver<ToolApproval>) {
    let (response_sender, response_receiver) = oneshot::channel();

    (
      Self {
        invocation,
        response_sender: Arc::new(Mutex::new(Some(response_sender))),
      },
      response_receiver,
    )
  }

  fn respond(&self, approval: ToolApproval) {
    if let Some(response_sender) = self
      .response_sender
      .lock()
      .expect("approval response lock poisoned")
      .take()
    {
      let _ = response_sender.send(approval);
    }
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
