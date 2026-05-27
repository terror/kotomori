use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan<'a> {
  Full {
    clear: bool,
  },
  NoOperation {
    previous: &'a PresentedFrame,
    previous_viewport: Viewport,
  },
  Patch {
    previous: &'a PresentedFrame,
    previous_viewport: Viewport,
    patch: PatchPlan,
  },
}

impl RenderPlan<'_> {
  pub(crate) fn clears(&self) -> bool {
    matches!(self, Self::Full { clear: true })
  }
}
