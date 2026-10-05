use super::*;

#[derive(Debug, Default)]
pub(crate) struct ScrollState {
  pub(crate) offset: usize,
}

impl ScrollState {
  pub(crate) fn update(
    &mut self,
    focus: Range<usize>,
    height: usize,
    rows: usize,
  ) {
    if height == 0 {
      return;
    }

    let start = focus.start.min(rows.saturating_sub(1));

    let end = focus.end.min(rows).min(start.saturating_add(height));

    self.offset = self
      .offset
      .min(start)
      .max(end.saturating_sub(height))
      .min(rows.saturating_sub(height));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn follows_focus_and_clamps_after_resizing() {
    #[track_caller]
    fn case(
      scroll: &mut ScrollState,
      focus: Range<usize>,
      height: usize,
      rows: usize,
      expected: usize,
    ) {
      scroll.update(focus, height, rows);

      assert_eq!(scroll.offset, expected);
    }

    let mut scroll = ScrollState::default();

    case(&mut scroll, 0..1, 3, 5, 0);
    case(&mut scroll, 2..3, 3, 5, 0);
    case(&mut scroll, 3..4, 3, 5, 1);
    case(&mut scroll, 2..3, 3, 5, 1);
    case(&mut scroll, 0..1, 3, 5, 0);
    case(&mut scroll, 4..5, 3, 5, 2);
    case(&mut scroll, 2..3, 0, 5, 2);
    case(&mut scroll, 2..3, 3, 5, 2);
    case(&mut scroll, 3..4, 1, 5, 3);
    case(&mut scroll, 3..4, 4, 5, 1);
    case(&mut scroll, 1..2, 4, 2, 0);
    case(&mut scroll, 0..0, 4, 0, 0);
    case(&mut scroll, 4..6, 4, 8, 2);
    case(&mut scroll, 1..4, 4, 8, 1);
    case(&mut scroll, 4..8, 2, 8, 4);
    case(&mut scroll, 4..8, 4, 8, 4);
    case(&mut scroll, 20..21, 3, 6, 3);
    case(&mut scroll, 0..0, 3, 0, 0);
  }
}
