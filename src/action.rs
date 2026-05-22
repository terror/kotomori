use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
  Backspace,
  CompleteCommand,
  Input(char),
  Quit,
  SelectNextCommand,
  SelectPreviousCommand,
  Submit,
}

impl Action {
  pub(crate) fn from_key(key: &KeyEvent) -> Option<Self> {
    match key.code {
      KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        Some(Self::Quit)
      }
      KeyCode::Enter => Some(Self::Submit),
      KeyCode::Backspace => Some(Self::Backspace),
      KeyCode::Tab => Some(Self::CompleteCommand),
      KeyCode::Down => Some(Self::SelectNextCommand),
      KeyCode::Up => Some(Self::SelectPreviousCommand),
      KeyCode::Char(c) => Some(Self::Input(c)),
      _ => None,
    }
  }
}
