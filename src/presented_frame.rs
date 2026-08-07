use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentedFrame {
  pub(crate) cursor_row: usize,
  pub(crate) frame: Frame,
  pub(crate) viewport: Viewport,
}

impl PresentedFrame {
  pub(crate) fn new(
    cursor_row: usize,
    frame: Frame,
    viewport: Viewport,
  ) -> Self {
    Self {
      cursor_row,
      frame,
      viewport,
    }
  }
}

impl From<Frame> for PresentedFrame {
  fn from(frame: Frame) -> Self {
    Self {
      cursor_row: frame.last_row(),
      viewport: Viewport::anchored_to_bottom(
        frame.len(),
        frame.dimensions.height,
      ),
      frame,
    }
  }
}
