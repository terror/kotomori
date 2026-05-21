use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
  Backspace,
  Input(char),
  None,
  Quit,
  Submit,
}

impl From<&KeyEvent> for Action {
  fn from(key: &KeyEvent) -> Self {
    match key.code {
      KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        Self::Quit
      }
      KeyCode::Enter => Self::Submit,
      KeyCode::Backspace => Self::Backspace,
      KeyCode::Char(c) => Self::Input(c),
      _ => Self::None,
    }
  }
}
