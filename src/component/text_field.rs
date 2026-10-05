use super::*;

#[derive(Debug)]
pub(crate) struct TextFieldComponent<'a> {
  pub(crate) label: LineComponent,
  pub(crate) value: &'a str,
}

impl Component for TextFieldComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if width == 0 {
      return vec![LineComponent::blank()];
    }

    let label = self.label.clip_end(width - 1);

    let remaining = width - 1 - u16::try_from(label.width()).unwrap();

    label
      .append(LineComponent::raw(self.value).clip_start(remaining))
      .append(Span::styled(" ", Style::Selection))
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
        TextFieldComponent {
          label: LineComponent::styled(label, Style::Muted),
          value: ""
        }
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
        TextFieldComponent {
          label: LineComponent::styled("foo: ", Style::Muted),
          value
        }
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

  #[test]
  fn preserves_label_styles() {
    assert_eq!(
      TextFieldComponent {
        label: LineComponent::styled("foo", Style::Accent)
          .append(Span::styled(": ", Style::Muted)),
        value: "foobar"
      }
      .render(9),
      [LineComponent::from([
        Span::styled("foo", Style::Accent),
        Span::styled(": ", Style::Muted),
        Span::raw("bar"),
        Span::styled(" ", Style::Selection),
      ])],
    );
  }
}
