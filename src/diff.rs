use super::*;

/// A frame difference together with enough length information to render it.
///
/// [`ChangedRange`] identifies the dirty logical rows, but it does not say how
/// much of the previous frame disappeared. The renderer needs both pieces of
/// information so that ordinary rewrites, appended rows, and deleted tails can
/// be handled by separate terminal operations.
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

  pub(crate) fn is_pure_tail_delete(self) -> bool {
    self.changed.first >= self.next_len
  }

  /// Return the changed rows that can be written from the next frame.
  ///
  /// This is `None` when the next frame has no row corresponding to the first
  /// changed row. In practice that means either the next frame is empty, or the
  /// diff only deletes rows from the old tail and must be cleared instead of
  /// overwritten.
  pub(crate) fn writable_range(self) -> Option<RangeInclusive<usize>> {
    if self.next_len == 0 || self.changed.first >= self.next_len {
      return None;
    }

    Some(self.changed.first..=self.changed.last.min(self.next_len - 1))
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

  #[test]
  fn is_pure_tail_delete_is_false_for_inserted_tail() {
    assert!(
      !Diff::between(&frame(&["foo"]), &frame(&["foo", "bar"]))
        .unwrap()
        .is_pure_tail_delete(),
    );
  }

  #[test]
  fn is_pure_tail_delete_is_false_for_middle_change_with_tail_delete() {
    assert!(
      !Diff::between(&frame(&["foo", "bar", "baz"]), &frame(&["foo", "qux"]))
        .unwrap()
        .is_pure_tail_delete(),
    );
  }

  #[test]
  fn is_pure_tail_delete_is_true_when_only_suffix_was_removed() {
    assert!(
      Diff::between(&frame(&["foo", "bar", "baz"]), &frame(&["foo"]))
        .unwrap()
        .is_pure_tail_delete(),
    );
  }

  #[test]
  fn writable_range_clamps_last_changed_row_to_last_next_row() {
    assert_eq!(
      Diff::between(&frame(&["foo", "bar", "baz"]), &frame(&["foo", "qux"]))
        .unwrap()
        .writable_range(),
      Some(1..=1),
    );
  }

  #[test]
  fn writable_range_is_none_for_empty_next_frame() {
    assert_eq!(
      Diff::between(&frame(&["foo"]), &frame(&[]))
        .unwrap()
        .writable_range(),
      None,
    );
  }

  #[test]
  fn writable_range_is_none_for_pure_tail_delete() {
    assert_eq!(
      Diff::between(&frame(&["foo", "bar", "baz"]), &frame(&["foo"]))
        .unwrap()
        .writable_range(),
      None,
    );
  }

  #[test]
  fn writable_range_returns_changed_rows_present_in_next_frame() {
    assert_eq!(
      Diff::between(
        &frame(&["foo", "bar", "baz"]),
        &frame(&["foo", "qux", "baz"])
      )
      .unwrap()
      .writable_range(),
      Some(1..=1),
    );
  }
}
