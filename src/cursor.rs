use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Cursor {
  row: usize,
}

impl Cursor {
  pub(crate) fn diff_to(
    self,
    from: Viewport,
    target_row: usize,
    to: Viewport,
  ) -> isize {
    let current_screen_row = from.screen_row(self.row);
    let target_screen_row = to.screen_row(target_row);

    isize::try_from(target_screen_row).unwrap_or(isize::MAX)
      - isize::try_from(current_screen_row).unwrap_or(isize::MAX)
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
      Cursor::new(10).diff_to(Viewport::new(8, 24), 12, Viewport::new(8, 24)),
      2,
    );
    assert_eq!(
      Cursor::new(12).diff_to(Viewport::new(8, 24), 10, Viewport::new(8, 24)),
      -2,
    );
    assert_eq!(
      Cursor::new(12).diff_to(Viewport::new(10, 24), 14, Viewport::new(12, 24)),
      0,
    );
  }
}
