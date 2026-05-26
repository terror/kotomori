use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RenderOperation {
  Full { clear: bool },
  NoOperation,
  Patch { diff: Diff },
}
