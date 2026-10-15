use super::*;

#[derive(Debug)]
pub(crate) struct ErrorComponent<'a> {
  pub(crate) error: &'a str,
}

impl Component for ErrorComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    StackComponent::default()
      .push(LineComponent::from([
        Span::styled("●", Style::Danger),
        Span::raw(" "),
        Span::raw("Error"),
      ]))
      .push(
        LinesComponent::raw(self.error.lines())
          .gutter(Span::styled("  │ ", Style::Muted)),
      )
      .render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wraps_details_inside_gutter() {
    assert_eq!(
      ErrorComponent { error: "foobar" }.render(8),
      [
        LineComponent::from([
          Span::styled("●", Style::Danger),
          Span::raw(" "),
          Span::raw("Error"),
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
