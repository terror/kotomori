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

  pub(crate) fn commands(&self) -> impl Iterator<Item = Command> + '_ {
    Command::iter().filter(|command| command.matches(&self.input))
  }

  pub(crate) fn complete_command(&mut self) {
    if let Some(command) = self.selected_command() {
      self.input = command.input();
    }
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

impl Component for Composer {
  fn render(&self, _width: u16) -> Vec<Line> {
    let mut lines = vec![
      vec![
        Span::raw("  "),
        Span::styled("❯ ", Style::CyanBold),
        Span::raw(self.input_text().to_string()),
      ]
      .into(),
    ];

    let selected = self.selected_command_index();

    lines.extend(self.commands().enumerate().map(|(index, command)| {
      let style = if Some(index) == selected {
        Style::CyanBold
      } else {
        Style::Gray
      };

      let prefix = if Some(index) == selected {
        "  ❯ "
      } else {
        "    "
      };

      vec![
        Span::raw(prefix),
        Span::styled(command.input(), style),
        Span::styled("  ", Style::DarkGray),
        Span::styled(command.description(), Style::DarkGray),
      ]
      .into()
    }));

    lines
  }
}
