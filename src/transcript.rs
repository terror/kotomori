use super::*;

#[derive(Debug)]
pub(crate) struct Transcript<'a> {
  state: &'a State,
}

impl<'a> Transcript<'a> {
  pub(crate) fn new(state: &'a State) -> Self {
    Self { state }
  }
}

impl Widget for Transcript<'_> {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let lines = self
      .state
      .messages()
      .iter()
      .flat_map(Message::lines)
      .collect::<Vec<_>>();

    Paragraph::new(lines)
      .wrap(Wrap { trim: false })
      .render(area, buf);
  }
}
