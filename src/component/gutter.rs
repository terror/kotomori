use super::*;

#[derive(Debug)]
pub(crate) struct GutterComponent<C> {
  pub(crate) component: C,
  pub(crate) gutter: LineComponent,
}

impl<C: Component> Component for GutterComponent<C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let content_width = width
      .saturating_sub(u16::try_from(self.gutter.width()).unwrap_or(u16::MAX));

    let gutter = self.gutter.clip_end(width);

    self
      .component
      .render(content_width)
      .into_iter()
      .map(|line| gutter.clone().append(line))
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
      assert_eq!(
        GutterComponent {
          component: LineComponent::raw("foo"),
          gutter: LineComponent::styled(input, Style::Muted)
        }
        .render(width),
        [LineComponent::styled(gutter, Style::Muted)],
      );
    }

    case("  │ ", 0, "");
    case("  │ ", 1, " ");
    case("  │ ", 2, "  ");
    case("  │ ", 3, "  │");
    case("  │ ", 4, "  │ ");
    case("#\u{fe0f}", 1, "");
    case("👩‍💻", 1, "");
    case("界", 1, "");
  }

  #[test]
  fn wraps_lines_with_a_styled_gutter() {
    assert_eq!(
      GutterComponent {
        component: LinesComponent::raw(["foobar"]),
        gutter: LineComponent::styled("│", Style::Accent)
          .append(Span::styled(" ", Style::Muted))
      }
      .render(5),
      [
        LineComponent::from([
          Span::styled("│", Style::Accent),
          Span::styled(" ", Style::Muted),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("│", Style::Accent),
          Span::styled(" ", Style::Muted),
          Span::raw("bar"),
        ]),
      ]
    );
  }
}
