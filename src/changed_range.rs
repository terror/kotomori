use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChangedRange {
  pub(crate) first: usize,
  pub(crate) last: usize,
}

impl ChangedRange {
  pub(crate) fn between(previous: &[String], next: &[String]) -> Option<Self> {
    let mut changed = None;

    for index in 0..previous.len().max(next.len()) {
      let (previous, next) = (
        previous.get(index).map_or("", String::as_str),
        next.get(index).map_or("", String::as_str),
      );

      if previous != next {
        changed = Some(match changed {
          Some(Self { first, .. }) => Self { first, last: index },
          None => Self {
            first: index,
            last: index,
          },
        });
      }
    }

    changed
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_first_and_last_changed_line() {
    assert_eq!(
      ChangedRange::between(
        &["foo".into(), "bar".into(), "baz".into()],
        &["foo".into(), "qux".into(), "baz".into(), "bob".into()],
      ),
      Some(ChangedRange { first: 1, last: 3 }),
    );
  }
}
