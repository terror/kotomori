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
  fn viewport_top() {
    #[track_caller]
    fn case(lines: usize, height: usize, expected: usize) {
      let frame = Frame::new(
        vec![String::new(); lines],
        Dimensions { height, width: 80 },
      );

      assert_eq!(frame.viewport_top, expected);
    }

    case(10, 10, 0);
    case(3, 10, 0);
    case(30, 10, 20);
    case(10, 0, 10);
  }
}
