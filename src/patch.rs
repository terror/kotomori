use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Patch {
  pub(crate) changed: ChangedRange,
  pub(crate) move_target_row: usize,
  pub(crate) prepend_line_feed: bool,
  pub(crate) viewport_top: usize,
}
