use super::*;

#[derive(Debug)]
pub(crate) struct Composer {
  command_index: usize,
  footer: Option<Footer>,
  textarea: TextArea<'static>,
}

impl Composer {
  pub(crate) fn clear(&mut self) {
    self.textarea = TextArea::default();
    self.command_index = 0;
  }

  fn command_input(&self) -> Option<&str> {
    let [input] = self.textarea.lines() else {
      return None;
    };

    Some(input)
  }

  pub(crate) fn commands(&self) -> impl Iterator<Item = Command> + '_ {
    Command::iter().filter(|command| {
      self
        .command_input()
        .is_some_and(|input| command.matches(input))
    })
  }

  pub(crate) fn complete_command(&mut self) -> bool {
    if let Some(command) = self.selected_command() {
      let input = command.input();
      self.set_input(&input);
      true
    } else {
      false
    }
  }

  pub(crate) fn footer(self, footer: Footer) -> Self {
    let footer = Some(footer);

    Self { footer, ..self }
  }

  pub(crate) fn input(&mut self, input: Input) {
    if self.textarea.input(input) {
      self.command_index = 0;
    }
  }

  pub(crate) fn input_text(&self) -> String {
    self.textarea.lines().join("\n")
  }

  pub(crate) fn new(input: &str) -> Self {
    Self {
      command_index: 0,
      footer: None,
      textarea: Self::textarea(input),
    }
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

  fn set_input(&mut self, input: &str) {
    self.textarea = Self::textarea(input);
    self.command_index = 0;
  }

  fn textarea(input: &str) -> TextArea<'static> {
    let mut textarea = TextArea::from(
      input
        .split('\n')
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
    );

    textarea.move_cursor(CursorMove::Bottom);
    textarea.move_cursor(CursorMove::End);

    textarea
  }
}

impl Component for Composer {
  fn render(&self, width: u16) -> Vec<Line> {
    let cursor = self.textarea.cursor();

    let selected = self.selected_command_index();

    let mut lines = FramedLines::new(
      self.textarea.lines().iter().enumerate().map(|(row, line)| {
        if cursor.0 != row {
          return Line::raw(line);
        }

        let mut chars = line.chars();

        let before = chars.by_ref().take(cursor.1).collect::<String>();
        let under_cursor = chars.next().unwrap_or(' ');
        let after = chars.collect::<String>();

        Line::from(vec![
          Span::raw(before),
          Span::styled(under_cursor.to_string(), Style::Reverse),
          Span::raw(after),
        ])
      }),
    )
    .render(width);

    if selected.is_none() {
      lines.extend(self.footer.iter().flat_map(|footer| footer.render(width)));
    }

    lines.extend(self.commands().enumerate().map(|(index, command)| {
      let input_style = match selected {
        Some(selected) if selected == index => Style::CyanBold,
        _ => Style::Gray,
      };

      Line::from(vec![
        Span::styled(command.input(), input_style),
        Span::styled("  ", Style::DarkGray),
        Span::styled(command.description(), Style::DarkGray),
      ])
    }));

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn command_rendering() {
    let lines = Composer::new("/").render(80);

    assert_eq!(
      lines[3],
      vec![
        Span::styled("/clear", Style::CyanBold),
        Span::styled("  ", Style::DarkGray),
        Span::styled("Clear the transcript", Style::DarkGray),
      ]
      .into()
    );

    assert_eq!(
      lines[4],
      vec![
        Span::styled("/quit", Style::Gray),
        Span::styled("  ", Style::DarkGray),
        Span::styled("Quit kotomori", Style::DarkGray),
      ]
      .into()
    );
  }

  #[test]
  fn footer_is_hidden_when_command_menu_is_open() {
    let composer = Composer::new("/").footer(Footer::raw("foo"));

    assert!(
      !composer
        .render(80)
        .contains(&vec![Span::styled("foo", Style::DarkGray)].into())
    );
  }

  #[test]
  fn footer_rendering() {
    let composer = Composer::new("foo").footer(Footer::raw("bar"));

    assert_eq!(
      composer.render(80)[3],
      vec![Span::styled("bar", Style::DarkGray)].into()
    );
  }
}
