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
  fn truncate() {
    #[track_caller]
    fn case(text: &str, len: usize, expected: &str) {
      assert_eq!(text.truncate(len), expected);
    }

    case("foo", 3, "foo");
    case("foo", 4, "foo");
    case("foobarbaz", 3, "foo...");
    case("åéî", 2, "åé...");
    case("", 0, "");
    case("foo", 0, "...");
  }
}
