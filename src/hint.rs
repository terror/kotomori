use super::*;

#[derive(Debug)]
pub(crate) struct Hint;

impl Widget for Hint {
  fn render(self, area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(vec![
      Span::styled(
        "  Type a prompt. Press ",
        Style::default().fg(Color::DarkGray),
      ),
      Span::styled("Ctrl-C", Style::default().fg(Color::Gray)),
      Span::styled(" to quit.", Style::default().fg(Color::DarkGray)),
    ]))
    .render(area, buf);
  }
}
