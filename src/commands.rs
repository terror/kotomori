use super::*;

#[derive(Debug)]
pub(crate) struct Commands<'a> {
  state: &'a State,
}

impl<'a> Commands<'a> {
  pub(crate) fn new(state: &'a State) -> Self {
    Self { state }
  }
}

impl Widget for Commands<'_> {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let selected = self.state.selected_command_index();

    let lines = self
      .state
      .commands()
      .enumerate()
      .map(|(index, command)| {
        let style = if Some(index) == selected {
          Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
        } else {
          Style::default().fg(Color::Gray)
        };

        let prefix = if Some(index) == selected {
          "  ❯ "
        } else {
          "    "
        };

        Line::from(vec![
          Span::raw(prefix),
          Span::styled(command.input(), style),
          Span::styled("  ", Style::default().fg(Color::DarkGray)),
          Span::styled(
            command.description(),
            Style::default().fg(Color::DarkGray),
          ),
        ])
      })
      .collect::<Vec<_>>();

    Paragraph::new(lines).render(area, buf);
  }
}
