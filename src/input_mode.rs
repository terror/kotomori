use super::*;

#[derive(Debug, Default)]
pub(crate) enum InputMode {
  Approval(ApprovalRequest),
  #[default]
  Compose,
}

impl InputMode {
  pub(crate) fn approval(&self) -> Option<&ApprovalRequest> {
    match self {
      Self::Approval(request) => Some(request),
      Self::Compose => None,
    }
  }

  pub(crate) fn clear_approval(&mut self) {
    if matches!(self, Self::Approval(_)) {
      *self = Self::Compose;
    }
  }

  pub(crate) fn resolve_approval(&mut self, approval: ToolApproval) {
    let Self::Approval(request) = mem::take(self) else {
      return;
    };

    match approval {
      ToolApproval::Approved => request.approve(),
      ToolApproval::Denied => request.deny(),
    }
  }
}
