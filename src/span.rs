use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Span {
  pub(crate) style: Style,
  pub(crate) text: String,
}

impl Span {
  fn new(text: impl Into<String>, style: Style) -> Self {
    let text = text.into();

    let text = if text.chars().any(char::is_control) {
      let mut escaped = String::with_capacity(text.len());

      for c in text.chars() {
        if c.is_control() {
          escaped.extend(c.escape_default());
        } else {
          escaped.push(c);
        }
      }

      escaped
    } else {
      text
    };

    Self { style, text }
  }

  pub(crate) fn push(&mut self, c: char) {
    if c.is_control() {
      self.text.extend(c.escape_default());
    } else {
      self.text.push(c);
    }
  }

  pub(crate) fn raw(text: impl Into<String>) -> Self {
    Self::new(text, Style::None)
  }

  pub(crate) fn styled(text: impl Into<String>, style: Style) -> Self {
    Self::new(text, style)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_terminal_controls_and_preserves_unicode() {
    let input =
      "foo\x1b[2J\x1b]52;c;Zm9v\x07\r\t\x08\u{009b}\u{009c}\u{009d}界";

    let expected = concat!(
      r"foo\u{1b}[2J\u{1b}]52;c;Zm9v\u{7}\r\t\u{8}",
      r"\u{9b}\u{9c}\u{9d}界",
    );

    for span in [Span::raw(input), Span::styled(input, Style::Accent)] {
      assert_eq!(span.text, expected);
    }
  }
}
