use super::*;

#[derive(Debug)]
pub(crate) struct Composer {
  command_index: usize,
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
      textarea: Self::textarea(input),
    }
  }

  fn render_input_line(line: &str, row: usize, cursor: DataCursor) -> Line {
    if cursor.0 != row {
      return Line::raw(line);
    }

    let col = cursor.1;
    let before = line.chars().take(col).collect::<String>();
    let under_cursor = line.chars().nth(col);
    let after = line.chars().skip(col.saturating_add(1)).collect::<String>();
    let mut spans = Vec::new();

    if !before.is_empty() {
      spans.push(Span::raw(before));
    }

    spans.push(Span::styled(
      under_cursor.unwrap_or(' ').to_string(),
      Style::Reverse,
    ));

    if !after.is_empty() {
      spans.push(Span::raw(after));
    }

    spans.into()
  }

  fn render_textarea(&self, width: u16) -> Vec<Line> {
    let cursor = self.textarea.cursor();

    Self::render_textarea_lines(
      self
        .textarea
        .lines()
        .iter()
        .enumerate()
        .map(|(row, line)| Self::render_input_line(line, row, cursor)),
      width,
    )
  }

  pub(crate) fn render_textarea_content<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    width: u16,
  ) -> Vec<Line> {
    Self::render_textarea_lines(lines.into_iter().map(Line::raw), width)
  }

  fn render_textarea_lines(
    lines: impl IntoIterator<Item = Line>,
    width: u16,
  ) -> Vec<Line> {
    let width = width.max(1);
    let border = "─".repeat(usize::from(width));

    let mut rendered =
      vec![vec![Span::styled(border.clone(), Style::DarkGray)].into()];

    for line in lines {
      rendered.extend(line.render(width));
    }

    rendered.push(vec![Span::styled(border, Style::DarkGray)].into());

    rendered
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
    let mut lines = self.render_textarea(width);

    let selected = self.selected_command_index();

    lines.extend(self.commands().enumerate().map(|(index, command)| {
      let style = if Some(index) == selected {
        Style::CyanBold
      } else {
        Style::Gray
      };

      vec![
        Span::raw("  "),
        Span::styled(command.input(), style),
        Span::styled("  ", Style::DarkGray),
        Span::styled(command.description(), Style::DarkGray),
      ]
      .into()
    }));

    lines
  }
}
