use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full { clear: bool },
  NoOperation { viewport: Viewport },
  Patch { viewport: Viewport, diff: Diff },
}
