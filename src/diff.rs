use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Diff {
  pub(crate) changed: ChangedRange,
  next_len: usize,
  previous_len: usize,
}

impl Diff {
  pub(crate) fn between(previous: &Frame, next: &Frame) -> Option<Self> {
    Some(Self {
      changed: ChangedRange::between(&previous.lines, &next.lines)?,
      next_len: next.len(),
      previous_len: previous.len(),
    })
  }

  pub(crate) fn deleted_tail_len(self) -> usize {
    self.previous_len.saturating_sub(self.next_len)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn frame(lines: &[&str]) -> Frame {
    Frame::new(
      lines.iter().map(|line| (*line).into()).collect(),
      Dimensions {
        height: 24,
        width: 80,
      },
    )
  }

  #[test]
  fn between_returns_none_when_frames_are_identical() {
    assert_eq!(
      Diff::between(
        &frame(&["foo", "bar", "baz"]),
        &frame(&["foo", "bar", "baz"])
      ),
      None,
    );
  }

  #[test]
  fn between_tracks_changed_range_and_lengths_for_middle_change() {
    assert_eq!(
      Diff::between(
        &frame(&["foo", "bar", "baz"]),
        &frame(&["foo", "qux", "baz"])
      ),
      Some(Diff {
        changed: ChangedRange { first: 1, last: 1 },
        previous_len: 3,
        next_len: 3,
      }),
    );
  }

  #[test]
  fn deleted_tail_len_is_zero_when_next_is_longer() {
    assert_eq!(
      Diff::between(&frame(&["foo"]), &frame(&["foo", "bar"]))
        .unwrap()
        .deleted_tail_len(),
      0,
    );
  }

  #[test]
  fn deleted_tail_len_is_zero_when_next_is_same_length() {
    assert_eq!(
      Diff::between(
        &frame(&["foo", "bar", "baz"]),
        &frame(&["foo", "qux", "baz"])
      )
      .unwrap()
      .deleted_tail_len(),
      0,
    );
  }

  #[test]
  fn deleted_tail_len_returns_removed_suffix_length() {
    assert_eq!(
      Diff::between(
        &frame(&["foo", "bar", "baz", "qux"]),
        &frame(&["foo", "bar"])
      )
      .unwrap()
      .deleted_tail_len(),
      2,
    );
  }
}
