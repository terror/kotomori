use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
  CompleteCommand,
  Edit(Input),
  Interrupt,
  Quit,
  SelectNext,
  SelectPrevious,
  Submit,
}

impl Action {
  pub(crate) fn from_key(key: &KeyEvent) -> Self {
    match key.code {
      KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        Self::Quit
      }
      KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        Self::Edit(Input {
          key: Key::Enter,
          ..Default::default()
        })
      }
      KeyCode::Esc => Self::Interrupt,
      KeyCode::Enter if key.modifiers.is_empty() => Self::Submit,
      KeyCode::Tab => Self::CompleteCommand,
      KeyCode::Down => Self::SelectNext,
      KeyCode::Up => Self::SelectPrevious,
      _ => Self::Edit((*key).into()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ctrl_j_inserts_newline() {
    assert_eq!(
      Action::from_key(&KeyEvent::new(
        KeyCode::Char('j'),
        KeyModifiers::CONTROL
      )),
      Action::Edit(Input {
        key: Key::Enter,
        ..Default::default()
      })
    );
  }

  #[test]
  fn esc_interrupts() {
    assert_eq!(
      Action::from_key(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
      Action::Interrupt
    );
  }
}
