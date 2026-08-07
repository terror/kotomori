use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentedFrame {
  pub(crate) frame: Frame,
  pub(crate) viewport: Viewport,
}

impl PresentedFrame {
  pub(crate) fn new(frame: Frame, viewport: Viewport) -> Self {
    Self { frame, viewport }
  }
}

impl From<Frame> for PresentedFrame {
  fn from(frame: Frame) -> Self {
    Self {
      viewport: Viewport::anchored_to_bottom(
        frame.len(),
        frame.dimensions.height,
      ),
      frame,
    }
  }
}
