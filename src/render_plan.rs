use super::*;

/// The renderer's decision for turning a previous frame into the next one.
///
/// Full renders are used when terminal state is too uncertain or when a patch
/// would need to write outside the visible viewport. Patches are used when the
/// changed logical rows can be reached from the recorded cursor and viewport.
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
