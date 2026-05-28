use super::*;

pub(crate) trait StrExt {
  fn truncate(&self, len: usize) -> String;
}

impl StrExt for str {
  fn truncate(&self, len: usize) -> String {
    let mut chars = self.chars();

    let text = chars.by_ref().take(len).collect::<String>();

    if chars.next().is_some() {
      format!("{text}...")
    } else {
      text
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn leaves_short_strings_alone() {
    assert_eq!("foo".truncate(3), "foo");
    assert_eq!("foo".truncate(4), "foo");
  }

  #[test]
  fn truncates_long_strings() {
    assert_eq!("foobarbaz".truncate(3), "foo...");
  }

  #[test]
  fn truncates_unicode_on_character_boundaries() {
    assert_eq!("åéî".truncate(2), "åé...");
  }

  #[test]
  fn zero_length_truncates_non_empty_strings() {
    assert_eq!("".truncate(0), "");
    assert_eq!("foo".truncate(0), "...");
  }
}
