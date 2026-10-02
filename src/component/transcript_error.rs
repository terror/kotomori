use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptErrorComponent<'a> {
  error: &'a str,
}

impl<'a> TranscriptErrorComponent<'a> {
  const GUTTER: &'static str = "  │ ";

  pub(crate) fn new(error: &'a str) -> Self {
    Self { error }
  }
}

impl Component for TranscriptErrorComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = LineComponent::from([
      Span::styled("●", Style::Danger),
      Span::raw(" "),
      Span::raw("Error"),
    ])
    .wrap(width);

    lines.extend(
      GutteredLinesComponent::raw(self.error.lines())
        .with_gutter(Span::styled(Self::GUTTER, Style::Muted))
        .render(width),
    );

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
          Span::styled("●", Style::Danger),
          Span::raw(" Error"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
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
          Span::styled("●", Style::Danger),
          Span::raw(" Error"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("foob"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("ar"),
        ]),
      ]
    );
  }
}
