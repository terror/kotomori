use super::*;

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
