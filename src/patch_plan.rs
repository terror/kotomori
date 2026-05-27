use super::*;

/// A terminal patch that can be applied without redrawing the whole frame.
///
/// Tail deletion is separated from ordinary updates because deleted rows do
/// not have replacement text in the next frame. They have to be cleared on the
/// terminal after positioning the cursor relative to the previous viewport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PatchPlan {
  ClearDeletedTail {
    diff: Diff,
  },
  Update {
    diff: Diff,
    writable_range: RangeInclusive<usize>,
  },
}
