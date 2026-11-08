use super::*;

#[derive(Debug)]
pub(crate) struct PendingApproval {
  pub(crate) response_sender: oneshot::Sender<ToolApproval>,
  pub(crate) tool_id: ToolId,
}

#[derive(Clone, Debug)]
pub struct RunControl {
  pub(crate) pending: Arc<Mutex<Option<PendingApproval>>>,
  run_id: RunId,
}

impl RunControl {
  pub(crate) fn new() -> Self {
    Self {
      pending: Arc::default(),
      run_id: RunId::next(),
    }
  }

  #[must_use]
  pub fn respond(&self, response: ApprovalResponse) -> bool {
    if response.run_id != self.run_id {
      return false;
    }

    let mut pending = self.pending.lock().unwrap();

    if pending
      .as_ref()
      .is_none_or(|pending| pending.tool_id != response.tool_id)
    {
      return false;
    }

    pending
      .take()
      .unwrap()
      .response_sender
      .send(response.approval)
      .is_ok()
  }

  #[must_use]
  pub fn run_id(&self) -> RunId {
    self.run_id
  }
}
