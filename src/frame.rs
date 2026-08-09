use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Frame {
  pub(crate) dimensions: Dimensions,
  pub(crate) lines: Vec<String>,
  pub(crate) viewport_top: usize,
}

impl Frame {
  pub(crate) fn last_row(&self) -> usize {
    self.lines.len().saturating_sub(1)
  }

  pub(crate) fn len(&self) -> usize {
    self.lines.len()
  }

  pub(crate) fn new(lines: Vec<String>, dimensions: Dimensions) -> Self {
    let viewport_top = lines.len().saturating_sub(dimensions.height);

    Self {
      dimensions,
      lines,
      viewport_top,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exact_fit_starts_at_zero() {
    let frame = Frame::new(
      vec![String::new(); 10],
      Dimensions {
        height: 10,
        width: 80,
      },
    );

    assert_eq!(frame.viewport_top, 0);
  }

  #[test]
  fn short_frame_starts_at_zero() {
    let frame = Frame::new(
      vec![String::new(); 3],
      Dimensions {
        height: 10,
        width: 80,
      },
    );

    assert_eq!(frame.viewport_top, 0);
  }

  #[test]
  fn tall_frame_starts_at_last_page() {
    let frame = Frame::new(
      vec![String::new(); 30],
      Dimensions {
        height: 10,
        width: 80,
      },
    );

    assert_eq!(frame.viewport_top, 20);
  }

  #[test]
  fn zero_height_starts_at_frame_len() {
    let frame = Frame::new(
      vec![String::new(); 10],
      Dimensions {
        height: 0,
        width: 80,
      },
    );

    assert_eq!(frame.viewport_top, 10);
  }
}
