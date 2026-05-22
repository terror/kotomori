use super::*;

#[derive(Debug)]
pub(crate) struct Header;

impl Widget for Header {
  fn render(self, area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(vec![
      Span::raw("  "),
      Span::styled(
        env!("CARGO_PKG_NAME"),
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("  "),
      Span::styled(
        env!("CARGO_PKG_VERSION"),
        Style::default().fg(Color::DarkGray),
      ),
    ]))
    .render(area, buf);
  }
}
