use super::*;

/// A fully rendered view of the application at one terminal size.
///
/// Rows are logical rows, not necessarily rows currently visible on screen.
/// The renderer compares successive frames and then decides whether those
/// logical rows can be patched in place or require a full redraw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Frame {
  pub(crate) dimensions: Dimensions,
  pub(crate) lines: Vec<String>,
}

impl Frame {
  pub(crate) fn is_empty(&self) -> bool {
    self.lines.is_empty()
  }

  pub(crate) fn last_row(&self) -> usize {
    self.lines.len().saturating_sub(1)
  }

  pub(crate) fn len(&self) -> usize {
    self.lines.len()
  }

  pub(crate) fn new(lines: Vec<String>, dimensions: Dimensions) -> Self {
    Self { dimensions, lines }
  }
}
