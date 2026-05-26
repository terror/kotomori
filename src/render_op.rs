use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RenderOp {
  Full { clear: bool },
  Noop,
  Patch { diff: Diff },
}
