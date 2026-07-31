use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChangedRange {
  pub(crate) first: usize,
  pub(crate) last: usize,
}

impl ChangedRange {
  pub(crate) fn between(previous: &[String], next: &[String]) -> Option<Self> {
    let len = previous.len().max(next.len());

    let changed = |index| {
      previous.get(index).map(String::as_str)
        != next.get(index).map(String::as_str)
    };

    Some(Self {
      first: (0..len).find(|&index| changed(index))?,
      last: (0..len).rfind(|&index| changed(index))?,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn lines(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).into()).collect()
  }

  #[test]
  fn between_detects_added_blank_tail() {
    assert_eq!(
      ChangedRange::between(&lines(&["foo"]), &lines(&["foo", ""])),
      Some(ChangedRange { first: 1, last: 1 }),
    );
  }

  #[test]
  fn between_detects_added_tail() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar"]),
        &lines(&["foo", "bar", "baz"])
      ),
      Some(ChangedRange { first: 2, last: 2 }),
    );
  }

  #[test]
  fn between_detects_changed_middle_span() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar", "baz", "qux"]),
        &lines(&["foo", "bob", "rob", "qux"]),
      ),
      Some(ChangedRange { first: 1, last: 2 }),
    );
  }

  #[test]
  fn between_detects_changed_prefix() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar", "baz"]),
        &lines(&["qux", "bar", "baz"])
      ),
      Some(ChangedRange { first: 0, last: 0 }),
    );
  }

  #[test]
  fn between_detects_changed_suffix() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar", "baz"]),
        &lines(&["foo", "bar", "qux"])
      ),
      Some(ChangedRange { first: 2, last: 2 }),
    );
  }

  #[test]
  fn between_detects_disjoint_changes_as_one_span() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar", "baz", "qux"]),
        &lines(&["bob", "bar", "baz", "rob"]),
      ),
      Some(ChangedRange { first: 0, last: 3 }),
    );
  }

  #[test]
  fn between_detects_empty_string_changed_to_non_empty_string() {
    assert_eq!(
      ChangedRange::between(&lines(&[""]), &lines(&["foo"])),
      Some(ChangedRange { first: 0, last: 0 }),
    );
  }

  #[test]
  fn between_detects_non_empty_string_changed_to_empty_string() {
    assert_eq!(
      ChangedRange::between(&lines(&["foo"]), &lines(&[""])),
      Some(ChangedRange { first: 0, last: 0 }),
    );
  }

  #[test]
  fn between_detects_removed_blank_tail() {
    assert_eq!(
      ChangedRange::between(&lines(&["foo", ""]), &lines(&["foo"])),
      Some(ChangedRange { first: 1, last: 1 }),
    );
  }

  #[test]
  fn between_detects_removed_tail() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar", "baz"]),
        &lines(&["foo", "bar"])
      ),
      Some(ChangedRange { first: 2, last: 2 }),
    );
  }

  #[test]
  fn between_returns_none_for_empty_inputs() {
    assert_eq!(ChangedRange::between(&lines(&[]), &lines(&[])), None);
  }

  #[test]
  fn between_returns_none_for_identical_inputs() {
    assert_eq!(
      ChangedRange::between(
        &lines(&["foo", "bar", "baz"]),
        &lines(&["foo", "bar", "baz"])
      ),
      None,
    );
  }
}
