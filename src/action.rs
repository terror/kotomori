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
  SubmitImmediately,
  ToggleReasoning,
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
      KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        Self::ToggleReasoning
      }
      KeyCode::Esc => Self::Interrupt,
      KeyCode::Enter if key.modifiers == KeyModifiers::ALT => {
        Self::SubmitImmediately
      }
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
  fn ctrl_t_toggles_reasoning() {
    assert_eq!(
      Action::from_key(&KeyEvent::new(
        KeyCode::Char('t'),
        KeyModifiers::CONTROL
      )),
      Action::ToggleReasoning
    );
  }
}
