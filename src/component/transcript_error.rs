use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptErrorComponent<'a> {
  error: &'a str,
}

impl<'a> TranscriptErrorComponent<'a> {
  const GUTTER: &'static str = "   │ ";

  pub(crate) fn new(error: &'a str) -> Self {
    Self { error }
  }
}

impl Component for TranscriptErrorComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = vec![LineComponent::from([
      Span::raw(" "),
      Span::styled("●", Style::Danger),
      Span::raw(" "),
      Span::raw("Error"),
    ])];

    let detail_width = width.saturating_sub(5).max(1);

    for detail in self.error.lines() {
      for line in LineComponent::raw(detail).render(detail_width) {
        let mut spans = vec![Span::styled(Self::GUTTER, Style::Muted)];
        spans.extend(Vec::<Span>::from(line));
        lines.push(LineComponent::from(spans));
      }
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn renders_error() {
    assert_eq!(
      TranscriptErrorComponent::new("foo\nbar").render(80),
      [
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::Danger),
          Span::raw(" "),
          Span::raw("Error"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::Muted),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::Muted),
          Span::raw("bar"),
        ]),
      ]
    );
  }

  #[test]
  fn wraps_details_inside_gutter() {
    assert_eq!(
      TranscriptErrorComponent::new("foobar").render(8),
      [
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::Danger),
          Span::raw(" "),
          Span::raw("Error"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::Muted),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::Muted),
          Span::raw("bar"),
        ]),
      ]
    );
  }
}
