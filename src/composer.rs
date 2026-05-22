use super::*;

#[derive(Debug)]
pub(crate) struct Composer {
  command_index: usize,
  input: String,
}

impl Composer {
  pub(crate) fn backspace(&mut self) {
    self.input.pop();
    self.command_index = 0;
  }

  pub(crate) fn clear(&mut self) {
    self.input.clear();
    self.command_index = 0;
  }

  pub(crate) fn command_height(&self) -> u16 {
    u16::try_from(self.commands().count()).unwrap_or(u16::MAX)
  }

  pub(crate) fn commands(&self) -> impl Iterator<Item = Command> + '_ {
    Command::iter().filter(|command| command.matches(&self.input))
  }

  pub(crate) fn complete_command(&mut self) {
    if let Some(command) = self.selected_command() {
      self.input = command.input();
    }
  }

  pub(crate) fn height(&self) -> u16 {
    self.command_height().saturating_add(1)
  }

  pub(crate) fn input_text(&self) -> &str {
    &self.input
  }

  pub(crate) fn new(input: String) -> Self {
    Self {
      command_index: 0,
      input,
    }
  }

  pub(crate) fn push(&mut self, c: char) {
    self.input.push(c);
    self.command_index = 0;
  }

  pub(crate) fn select_next_command(&mut self) {
    let len = self.commands().count();

    if len > 0 {
      self.command_index = self.command_index.saturating_add(1) % len;
    }
  }

  pub(crate) fn select_previous_command(&mut self) {
    let len = self.commands().count();

    if len > 0 {
      self.command_index = if self.command_index == 0 {
        len.saturating_sub(1)
      } else {
        self.command_index.saturating_sub(1)
      };
    }
  }

  pub(crate) fn selected_command(&self) -> Option<Command> {
    self.commands().nth(self.selected_command_index()?)
  }

  pub(crate) fn selected_command_index(&self) -> Option<usize> {
    let len = self.commands().count();

    if len == 0 {
      None
    } else {
      Some(self.command_index.min(len.saturating_sub(1)))
    }
  }
}

impl Widget for &Composer {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let [input_area, command_area] = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Length(1), Constraint::Min(0)])
      .areas(area);

    Paragraph::new(Line::from(vec![
      Span::raw("  "),
      Span::styled(
        "❯ ",
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw(self.input_text().to_string()),
    ]))
    .render(input_area, buf);

    let selected = self.selected_command_index();

    let lines = self
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

    Paragraph::new(lines).render(command_area, buf);
  }
}
