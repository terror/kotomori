use super::*;

#[derive(Debug)]
pub(crate) struct Composer<'a> {
  state: &'a State,
}

impl<'a> Composer<'a> {
  pub(crate) fn new(state: &'a State) -> Self {
    Self { state }
  }
}

impl Widget for Composer<'_> {
  fn render(self, area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(vec![
      Span::raw("  "),
      Span::styled(
        "❯ ",
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw(self.state.input_text().to_string()),
    ]))
    .render(area, buf);
  }
}
