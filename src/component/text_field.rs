use super::*;

#[derive(Debug)]
pub(crate) struct TextFieldComponent<'a> {
  label: Span,
  value: &'a str,
}

impl<'a> TextFieldComponent<'a> {
  pub(crate) fn new(label: Span, value: &'a str) -> Self {
    Self { label, value }
  }
}

impl Component for TextFieldComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if width == 0 {
      return vec![LineComponent::blank()];
    }

    let mut remaining = usize::from(width) - 1;

    let label = self
      .label
      .text
      .graphemes(true)
      .take_while(|grapheme| {
        let width = UnicodeWidthStr::width(*grapheme);

        if width > remaining {
          return false;
        }

        remaining -= width;

        true
      })
      .collect::<String>();

    let value = Span::raw(self.value);

    let mut start = value.text.len();

    for (index, grapheme) in value.text.grapheme_indices(true).rev() {
      let width = UnicodeWidthStr::width(grapheme);

      if width > remaining {
        break;
      }

      if width > 0 {
        start = index;
      }

      remaining -= width;
    }

    LineComponent::from([
      Span::styled(label, self.label.style),
      Span::raw(&value.text[start..]),
      Span::styled(" ", Style::Selection),
    ])
    .render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clips_label_at_grapheme_boundaries() {
    #[track_caller]
    fn case(label: &str, width: u16, expected: &str) {
      assert_eq!(
        TextFieldComponent::new(Span::styled(label, Style::Muted), "")
          .render(width),
        LineComponent::from([
          Span::styled(expected, Style::Muted),
          Span::styled(" ", Style::Selection),
        ])
        .render(width),
      );
    }

    case("界foo", 2, "");
    case("界foo", 3, "界");
    case("e\u{0301}foo", 2, "e\u{0301}");
    case("#\u{fe0f}foo", 2, "");
    case("#\u{fe0f}foo", 3, "#\u{fe0f}");
    case("👩‍💻foo", 2, "");
    case("👩‍💻foo", 3, "👩‍💻");
  }

  #[test]
  fn keeps_value_end_and_cursor_visible() {
    #[track_caller]
    fn case(value: &str, width: u16, label: &str, expected: &str) {
      assert_eq!(
        TextFieldComponent::new(Span::styled("foo: ", Style::Muted), value)
          .render(width),
        LineComponent::from([
          Span::styled(label, Style::Muted),
          Span::raw(expected),
          Span::styled(" ", Style::Selection),
        ])
        .render(width),
      );
    }

    case("bar", 9, "foo: ", "bar");
    case("foobarbaz", 9, "foo: ", "baz");
    case("foo界界", 9, "foo: ", "界");
    case("fooe\u{0301}", 7, "foo: ", "e\u{0301}");
    case("foo\t", 9, "foo: ", r"o\t");
    case("foo#\u{fe0f}", 8, "foo: ", "#\u{fe0f}");
    case("foo#\u{fe0f}", 7, "foo: ", "");
    case("foo👩‍💻", 8, "foo: ", "👩‍💻");
    case("foo👩‍💻", 7, "foo: ", "");
    case("fooe\u{0301}", 6, "foo: ", "");
    case("bar", 5, "foo:", "");
    case("bar", 1, "", "");
    case("bar", 0, "", "");
  }
}
