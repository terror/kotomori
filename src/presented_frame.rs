use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentedFrame {
  pub(crate) frame: Frame,
  pub(crate) viewport_top: usize,
}

impl PresentedFrame {
  pub(crate) fn new(frame: Frame, viewport_top: usize) -> Self {
    Self {
      frame,
      viewport_top,
    }
  }
}

impl From<Frame> for PresentedFrame {
  fn from(frame: Frame) -> Self {
    let viewport_top = frame.len().saturating_sub(frame.dimensions.height);

    Self {
      frame,
      viewport_top,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exact_fit_starts_at_zero() {
    let presented = PresentedFrame::from(Frame::new(
      vec![String::new(); 10],
      Dimensions {
        height: 10,
        width: 80,
      },
    ));

    assert_eq!(presented.viewport_top, 0);
  }

  #[test]
  fn short_frame_starts_at_zero() {
    let presented = PresentedFrame::from(Frame::new(
      vec![String::new(); 3],
      Dimensions {
        height: 10,
        width: 80,
      },
    ));

    assert_eq!(presented.viewport_top, 0);
  }

  #[test]
  fn tall_frame_starts_at_last_page() {
    let presented = PresentedFrame::from(Frame::new(
      vec![String::new(); 30],
      Dimensions {
        height: 10,
        width: 80,
      },
    ));

    assert_eq!(presented.viewport_top, 20);
  }

  #[test]
  fn zero_height_starts_at_frame_len() {
    let presented = PresentedFrame::from(Frame::new(
      vec![String::new(); 10],
      Dimensions {
        height: 0,
        width: 80,
      },
    ));

    assert_eq!(presented.viewport_top, 10);
  }
}
