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

  pub(crate) fn is_pure_tail_delete(self) -> bool {
    self.changed.first >= self.next_len
  }

  pub(crate) fn writable_range(
    self,
  ) -> Option<std::ops::RangeInclusive<usize>> {
    if self.next_len == 0 || self.changed.first >= self.next_len {
      return None;
    }

    Some(self.changed.first..=self.changed.last.min(self.next_len - 1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn splits_writable_and_deleted_tail_ranges() {
    let previous = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );
    let next = Frame::new(
      vec!["foo".into(), "qux".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );
    let diff = Diff::between(&previous, &next).unwrap();

    assert_eq!(diff.writable_range(), Some(1..=1));
    assert_eq!(diff.deleted_tail_len(), 1);
    assert!(!diff.is_pure_tail_delete());
  }
}
