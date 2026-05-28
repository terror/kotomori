use super::*;

pub(crate) trait StrExt {
  /// Return at most `len` characters from the start of the string.
  ///
  /// If the string contains more than `len` characters, the returned string is
  /// suffixed with `...` to indicate that text was omitted. The suffix is not
  /// included in `len`, so the returned string may contain up to `len + 3`
  /// bytes. Truncation is based on Unicode scalar values rather than bytes, so
  /// the returned string is always valid UTF-8 and never splits a character
  /// encoding.
  ///
  /// If the string contains `len` or fewer characters, it is returned unchanged
  /// aside from allocation into a new `String`. Passing `0` returns an empty
  /// string for empty input and `...` for non-empty input.
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
