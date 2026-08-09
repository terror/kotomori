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

  pub(crate) fn cursor(&self) -> (usize, usize) {
    let DataCursor(row, column) = self.textarea.cursor();

    (row, column)
  }

  pub(crate) fn input(&mut self, input: Input) {
    if self.textarea.input(input) {
      self.command_index = 0;
    }
  }

  pub(crate) fn input_text(&self) -> String {
    self.textarea.lines().join("\n")
  }

  pub(crate) fn lines(&self) -> &[String] {
    self.textarea.lines()
  }

  pub(crate) fn new(input: &str) -> Self {
    Self {
      command_index: 0,
      textarea: Self::textarea(input),
    }
  }

  pub(crate) fn select_next(&mut self) {
    if self.selected_command().is_some() {
      self.select_next_command();
    } else {
      self.input(Input {
        key: Key::Down,
        ..Default::default()
      });
    }
  }

  fn select_next_command(&mut self) {
    let len = self.commands().count();

    if len > 0 {
      self.command_index = self.command_index.saturating_add(1) % len;
    }
  }

  pub(crate) fn select_previous(&mut self) {
    if self.selected_command().is_some() {
      self.select_previous_command();
    } else {
      self.input(Input {
        key: Key::Up,
        ..Default::default()
      });
    }
  }

  fn select_previous_command(&mut self) {
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
