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
      .flat_map(|line| line.render(content_width))
      .map(|line| {
        let mut spans = Vec::<Span>::from(line);
        spans.insert(0, Span::styled("│ ", Style::Accent));
        LineComponent::from(spans)
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
