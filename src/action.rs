use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
  AgentDelta(String),
  AgentDone,
  Backspace,
  Input(char),
  Quit,
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
      KeyCode::Char(c) => Some(Self::Input(c)),
      _ => None,
    }
  }
}
