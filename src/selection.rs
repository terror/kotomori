pub(crate) fn next(index: usize, len: usize) -> usize {
  if len == 0 {
    index
  } else {
    index.saturating_add(1) % len
  }
}

pub(crate) fn previous(index: usize, len: usize) -> usize {
  if len == 0 {
    index
  } else if index == 0 {
    len.saturating_sub(1)
  } else {
    index.saturating_sub(1)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn next_selection() {
    for (index, len, expected) in [
      (0, 0, 0),
      (2, 0, 2),
      (0, 1, 0),
      (0, 3, 1),
      (2, 3, 0),
      (usize::MAX, usize::MAX, 0),
    ] {
      assert_eq!(next(index, len), expected);
    }
  }

  #[test]
  fn previous_selection() {
    for (index, len, expected) in [
      (0, 0, 0),
      (2, 0, 2),
      (0, 1, 0),
      (2, 3, 1),
      (0, 3, 2),
      (0, usize::MAX, usize::MAX - 1),
    ] {
      assert_eq!(previous(index, len), expected);
    }
  }
}
