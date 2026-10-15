use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChangedRange {
  pub(crate) first: usize,
  pub(crate) last: usize,
}

impl ChangedRange {
  pub(crate) fn between(
    previous: &[impl AsRef<str>],
    next: &[impl AsRef<str>],
  ) -> Option<Self> {
    let len = previous.len().max(next.len());

    let changed = |index| {
      previous.get(index).map(AsRef::as_ref)
        != next.get(index).map(AsRef::as_ref)
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

  #[test]
  fn between() {
    #[track_caller]
    fn case(previous: &[&str], next: &[&str], expected: Option<ChangedRange>) {
      assert_eq!(ChangedRange::between(previous, next), expected);
    }

    case(
      &["foo"],
      &["foo", ""],
      Some(ChangedRange { first: 1, last: 1 }),
    );
    case(
      &["foo", "bar"],
      &["foo", "bar", "baz"],
      Some(ChangedRange { first: 2, last: 2 }),
    );
    case(
      &["foo", "bar", "baz", "qux"],
      &["foo", "bob", "rob", "qux"],
      Some(ChangedRange { first: 1, last: 2 }),
    );
    case(
      &["foo", "bar", "baz"],
      &["qux", "bar", "baz"],
      Some(ChangedRange { first: 0, last: 0 }),
    );
    case(
      &["foo", "bar", "baz"],
      &["foo", "bar", "qux"],
      Some(ChangedRange { first: 2, last: 2 }),
    );
    case(
      &["foo", "bar", "baz", "qux"],
      &["bob", "bar", "baz", "rob"],
      Some(ChangedRange { first: 0, last: 3 }),
    );
    case(&[""], &["foo"], Some(ChangedRange { first: 0, last: 0 }));
    case(&["foo"], &[""], Some(ChangedRange { first: 0, last: 0 }));
    case(
      &["foo", ""],
      &["foo"],
      Some(ChangedRange { first: 1, last: 1 }),
    );
    case(
      &["foo", "bar", "baz"],
      &["foo", "bar"],
      Some(ChangedRange { first: 2, last: 2 }),
    );
    case(&[], &[], None);
    case(&["foo", "bar", "baz"], &["foo", "bar", "baz"], None);
  }
}
