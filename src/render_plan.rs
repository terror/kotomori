use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full { clear: bool },
  NoOperation { viewport: Viewport },
  Patch { viewport: Viewport, diff: Diff },
}

impl RenderPlan {
  pub(crate) fn clears(&self) -> bool {
    matches!(self, Self::Full { clear: true })
  }
}
