#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Viewport {
  height: usize,
  top: usize,
}

impl Viewport {
  pub(crate) fn anchored_to_bottom(frame_len: usize, height: usize) -> Self {
    Self {
      height,
      top: frame_len.max(height).saturating_sub(height),
    }
  }

  pub(crate) fn bottom(self) -> usize {
    self.top.saturating_add(self.height.saturating_sub(1))
  }

  pub(crate) fn height(self) -> usize {
    self.height
  }

  pub(crate) fn new(top: usize, height: usize) -> Self {
    Self { height, top }
  }

  pub(crate) fn screen_row(self, row: usize) -> usize {
    row.saturating_sub(self.top)
  }

  pub(crate) fn scrolled_down(self, rows: usize) -> Self {
    Self {
      height: self.height,
      top: self.top.saturating_add(rows),
    }
  }

  pub(crate) fn top(self) -> usize {
    self.top
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anchors_to_bottom() {
    assert_eq!(
      Viewport::anchored_to_bottom(30, 10),
      Viewport {
        height: 10,
        top: 20
      },
    );
    assert_eq!(
      Viewport::anchored_to_bottom(3, 10),
      Viewport { height: 10, top: 0 },
    );
  }
}
