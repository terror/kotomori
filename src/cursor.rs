use super::*;

/// The terminal cursor expressed as a logical row in the rendered frame.
///
/// This deliberately does not store a terminal screen row. A logical row only
/// becomes a physical cursor position when interpreted through a [`Viewport`],
/// which lets rendering code account for terminal scrollback between writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Cursor {
  row: usize,
}

impl Cursor {
  /// Return the signed vertical movement needed to reach `target_row`.
  ///
  /// The source and target rows may be interpreted through different
  /// viewports. This is what lets callers compute cursor movement across
  /// operations that scroll the terminal while preserving the logical row that
  /// the cursor is attached to.
  pub(crate) fn diff_to(
    self,
    from: Viewport,
    target_row: usize,
    to: Viewport,
  ) -> isize {
    isize::try_from(to.screen_row(target_row)).unwrap_or(isize::MAX)
      - isize::try_from(from.screen_row(self.row)).unwrap_or(isize::MAX)
  }

  pub(crate) fn new(row: usize) -> Self {
    Self { row }
  }

  pub(crate) fn row(self) -> usize {
    self.row
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn diff_accounts_for_viewports() {
    assert_eq!(
      Cursor::new(12).diff_to(Viewport::new(10, 24), 14, Viewport::new(12, 24)),
      0,
    );
  }

  #[test]
  fn diff_returns_negative_when_target_is_above_cursor() {
    assert_eq!(
      Cursor::new(12).diff_to(Viewport::new(8, 24), 10, Viewport::new(8, 24)),
      -2,
    );
  }

  #[test]
  fn diff_returns_positive_when_target_is_below_cursor() {
    assert_eq!(
      Cursor::new(10).diff_to(Viewport::new(8, 24), 12, Viewport::new(8, 24)),
      2,
    );
  }

  #[test]
  fn new_stores_row() {
    assert_eq!(Cursor::new(10).row(), 10);
  }
}
