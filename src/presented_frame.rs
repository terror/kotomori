use super::*;

/// The last frame known to have been presented to the terminal.
///
/// This stores both the logical frame and the terminal state left behind after
/// writing it. Keeping the cursor and viewport with the frame lets the next
/// draw decide whether it can patch from the current terminal position or must
/// clear and redraw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentedFrame {
  pub(crate) cursor: Cursor,
  pub(crate) frame: Frame,
  pub(crate) viewport: Viewport,
}

impl PresentedFrame {
  pub(crate) fn new(cursor: Cursor, frame: Frame, viewport: Viewport) -> Self {
    Self {
      cursor,
      frame,
      viewport,
    }
  }
}

impl From<Frame> for PresentedFrame {
  fn from(frame: Frame) -> Self {
    Self {
      cursor: Cursor::new(frame.last_row()),
      viewport: Viewport::anchored_to_bottom(
        frame.len(),
        frame.dimensions.height(),
      ),
      frame,
    }
  }
}
