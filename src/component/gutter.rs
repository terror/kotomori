use super::*;

#[derive(Debug)]
pub(crate) struct GutterComponent<C> {
  component: C,
  gutter: Span,
}

impl<C> GutterComponent<C> {
  pub(crate) fn new(component: C, gutter: Span) -> Self {
    Self { component, gutter }
  }
}

impl<C: Component> Component for GutterComponent<C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let content_width = width.saturating_sub(
      u16::try_from(UnicodeWidthStr::width(self.gutter.text.as_str()))
        .unwrap_or(u16::MAX),
    );

    let mut gutter = self.gutter.clone();

    while UnicodeWidthStr::width(gutter.text.as_str()) > usize::from(width) {
      gutter.text.pop();
    }

    self
      .component
      .render(content_width)
      .into_iter()
      .map(|mut line| {
        if !gutter.text.is_empty() {
          line.spans.insert(0, gutter.clone());
        }

        line
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clips_gutter_when_content_cannot_fit() {
    #[track_caller]
    fn case(input: &str, width: u16, gutter: &str) {
      let expected = if gutter.is_empty() {
        LineComponent::blank()
      } else {
        LineComponent::from([Span::styled(gutter, Style::Muted), Span::raw("")])
      };

      assert_eq!(
        GutterComponent::new(
          LineComponent::raw("foo"),
          Span::styled(input, Style::Muted),
        )
        .render(width),
        [expected],
      );
    }

    case("  │ ", 0, "");
    case("  │ ", 1, " ");
    case("  │ ", 2, "  ");
    case("  │ ", 3, "  │");
    case("  │ ", 4, "  │ ");
    case("#\u{fe0f}", 1, "#");
    case("界", 1, "");
  }

  #[test]
  fn wraps_lines_with_an_accent_gutter() {
    assert_eq!(
      GutterComponent::new(
        LinesComponent::raw(["foobar"]),
        Span::styled("│ ", Style::Accent),
      )
      .render(5),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("bar"),
        ]),
      ]
    );
  }
}
