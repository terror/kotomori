use super::*;

#[derive(Clone, Debug)]
pub struct AgentControl {
  pub(super) cancellation: CancellationToken,
  pending: Arc<Mutex<Option<PendingApproval>>>,
}

impl AgentControl {
  pub fn cancel(&self) {
    self.cancellation.cancel();
    self.deny_pending();
  }

  pub(super) fn deny_pending(&self) {
    let pending = self.pending.lock().unwrap().take();

    if let Some(pending) = pending {
      let _ = pending.response_sender.send(ToolApproval::Denied);
    }
  }

  pub(super) fn new() -> Self {
    Self {
      cancellation: CancellationToken::new(),
      pending: Arc::default(),
    }
  }

  pub(super) fn request(
    &self,
    id: ApprovalId,
  ) -> oneshot::Receiver<ToolApproval> {
    let (response_sender, response_receiver) = oneshot::channel();

    *self.pending.lock().unwrap() = Some(PendingApproval {
      id,
      response_sender,
    });

    response_receiver
  }

  #[must_use]
  pub fn resolve_approval(
    &self,
    id: &ApprovalId,
    approval: ToolApproval,
  ) -> bool {
    let pending = {
      let mut pending = self.pending.lock().unwrap();

      if self.cancellation.is_cancelled()
        || pending.as_ref().is_none_or(|pending| pending.id != *id)
      {
        return false;
      }

      pending.take().unwrap()
    };

    pending.response_sender.send(approval).is_ok()
  }
}
