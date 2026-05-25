use super::*;

#[derive(Debug)]
pub(crate) struct Hint;

impl Component for Hint {
  fn render(&self, _width: u16) -> Vec<Line> {
    vec![Line::from(vec![
      Span::styled("Type a prompt. Press ", Style::DarkGray),
      Span::styled("Ctrl-C", Style::Gray),
      Span::styled(" to quit.", Style::DarkGray),
    ])]
  }
}
