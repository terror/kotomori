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
