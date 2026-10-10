use super::*;

#[derive(Debug)]
pub(crate) struct Composer {
  command_index: usize,
  history: History,
  textarea: TextArea<'static>,
}

impl Composer {
  pub(crate) fn clear(&mut self) {
    self.textarea = TextArea::default();
    self.reset_navigation();
  }

  pub(crate) fn clear_history(&mut self) {
    self.history.clear();
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
      self.reset_navigation();
    }
  }

  pub(crate) fn input_text(&self) -> String {
    self.textarea.lines().join("\n")
  }

  pub(crate) fn lines(&self) -> &[String] {
    self.textarea.lines()
  }

  pub(crate) fn new(input: &str, history: Vec<String>) -> Self {
    Self {
      command_index: 0,
      history: History::new(history),
      textarea: Self::textarea(input),
    }
  }

  pub(crate) fn paste(&mut self, input: &str) {
    if input.is_empty() {
      return;
    }

    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    if self.textarea.insert_str(input) {
      self.reset_navigation();
    }
  }

  pub(crate) fn remember(&mut self, input: &str) {
    self.history.remember(input);
  }

  fn reset_navigation(&mut self) {
    self.command_index = 0;
    self.history.reset_navigation();
  }

  pub(crate) fn select_next(&mut self) {
    if self.selected_command().is_some() {
      self.select_next_command();
    } else if self.cursor().0 == self.lines().len().saturating_sub(1) {
      self.select_next_history();
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

  fn select_next_history(&mut self) {
    if let Some(input) = self.history.select_next() {
      self.set_input(&input);
    }
  }

  pub(crate) fn select_previous(&mut self) {
    if self.selected_command().is_some() {
      self.select_previous_command();
    } else if self.cursor().0 == 0 {
      self.select_previous_history();
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

  fn select_previous_history(&mut self) {
    if let Some(input) = self.history.select_previous(self.input_text()) {
      self.set_input(&input);
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
    let mut textarea = TextArea::from(input.split('\n'));

    textarea.move_cursor(CursorMove::Bottom);
    textarea.move_cursor(CursorMove::End);

    textarea
  }
}
