use super::*;

#[derive(Clone, Debug)]
pub(crate) struct GutteredLinesComponent {
  lines: Vec<LineComponent>,
}

impl GutteredLinesComponent {
  pub(crate) fn new(lines: impl IntoIterator<Item = LineComponent>) -> Self {
    Self {
      lines: lines.into_iter().collect(),
    }
  }

  pub(crate) fn raw<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
    Self::new(lines.into_iter().map(LineComponent::raw))
  }
}

impl Component for GutteredLinesComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let content_width = width.saturating_sub(2).max(1);

    self
      .lines
      .iter()
      .flat_map(|line| {
        line.render_prefixed(content_width, &Span::styled("│ ", Style::Accent))
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn render_preserves_blank_lines_and_wide_characters_at_narrow_widths() {
    for width in [0, 1, 2, 3] {
      assert_eq!(
        GutteredLinesComponent::raw(["", "界"]).render(width),
        [
          LineComponent::from([
            Span::styled("│ ", Style::Accent),
            Span::raw(""),
          ]),
          LineComponent::from([
            Span::styled("│ ", Style::Accent),
            Span::raw("界"),
          ]),
        ],
      );
    }
  }

  #[test]
  fn render_wraps_lines_with_an_accent_gutter() {
    assert_eq!(
      GutteredLinesComponent::raw(["foobar"]).render(5),
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
