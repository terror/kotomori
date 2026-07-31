#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Viewport {
  pub(crate) height: usize,
  pub(crate) top: usize,
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
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anchored_to_bottom_exact_fit_starts_at_zero() {
    assert_eq!(
      Viewport::anchored_to_bottom(10, 10),
      Viewport { height: 10, top: 0 },
    );
  }

  #[test]
  fn anchored_to_bottom_short_frame_starts_at_zero() {
    assert_eq!(
      Viewport::anchored_to_bottom(3, 10),
      Viewport { height: 10, top: 0 },
    );
  }

  #[test]
  fn anchored_to_bottom_tall_frame_starts_at_last_page() {
    assert_eq!(
      Viewport::anchored_to_bottom(30, 10),
      Viewport {
        height: 10,
        top: 20,
      },
    );
  }

  #[test]
  fn anchored_to_bottom_zero_height_starts_at_frame_len() {
    assert_eq!(
      Viewport::anchored_to_bottom(10, 0),
      Viewport { height: 0, top: 10 },
    );
  }

  #[test]
  fn bottom_one_height_equals_top() {
    assert_eq!(Viewport::new(20, 1).bottom(), 20);
  }

  #[test]
  fn bottom_saturates_at_usize_max() {
    assert_eq!(Viewport::new(usize::MAX - 2, 10).bottom(), usize::MAX,);
  }

  #[test]
  fn bottom_zero_height_equals_top() {
    assert_eq!(Viewport::new(20, 0).bottom(), 20);
  }

  #[test]
  fn new_sets_top_and_height() {
    assert_eq!(Viewport::new(7, 13), Viewport { top: 7, height: 13 },);
  }

  #[test]
  fn screen_row_above_viewport_saturates_to_zero() {
    assert_eq!(Viewport::new(20, 10).screen_row(19), 0);
  }

  #[test]
  fn screen_row_at_top_is_zero() {
    assert_eq!(Viewport::new(20, 10).screen_row(20), 0);
  }

  #[test]
  fn screen_row_below_top_is_offset_from_top() {
    assert_eq!(Viewport::new(20, 10).screen_row(29), 9);
  }

  #[test]
  fn scrolled_down_adds_rows_to_top() {
    assert_eq!(
      Viewport::new(20, 10).scrolled_down(5),
      Viewport {
        height: 10,
        top: 25,
      },
    );
  }

  #[test]
  fn scrolled_down_saturates_at_usize_max() {
    assert_eq!(
      Viewport::new(usize::MAX - 2, 10).scrolled_down(10),
      Viewport {
        height: 10,
        top: usize::MAX,
      },
    );
  }
}
