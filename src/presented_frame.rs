use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentedFrame {
  pub(crate) cursor: Cursor,
  pub(crate) frame: Frame,
  pub(crate) viewport: Viewport,
}

impl PresentedFrame {
  pub(crate) fn from_full_render(frame: Frame) -> Self {
    Self {
      cursor: Cursor::new(frame.last_row()),
      viewport: Viewport::anchored_to_bottom(
        frame.len(),
        frame.dimensions.height(),
      ),
      frame,
    }
  }

  pub(crate) fn new(cursor: Cursor, frame: Frame, viewport: Viewport) -> Self {
    Self {
      cursor,
      frame,
      viewport,
    }
  }
}
