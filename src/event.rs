use super::*;

#[derive(Debug, PartialEq)]
pub(crate) enum Event {
  Action(Action),
  Agent { event: AgentEvent, run_id: u64 },
  Error(String),
  Resize(Dimensions),
  Tick(Instant),
}

impl Event {
  pub(crate) fn from_terminal(event: &CrosstermEvent) -> Option<Self> {
    match event {
      CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
        Some(Self::Action(Action::from_key(key)))
      }
      CrosstermEvent::Resize(width, height) => Some(Self::Resize(Dimensions {
        height: usize::from(*height),
        width: *width,
      })),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn terminal_events() {
    #[track_caller]
    fn case(event: &CrosstermEvent, expected: Option<&Event>) {
      assert_eq!(Event::from_terminal(event).as_ref(), expected);
    }

    case(
      &CrosstermEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
      Some(&Event::Action(Action::Submit)),
    );
    case(
      &CrosstermEvent::Resize(80, 24),
      Some(&Event::Resize(Dimensions {
        height: 24,
        width: 80,
      })),
    );
    for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
      case(
        &CrosstermEvent::Key(KeyEvent::new_with_kind(
          KeyCode::Enter,
          KeyModifiers::NONE,
          kind,
        )),
        None,
      );
    }
    case(&CrosstermEvent::FocusGained, None);
  }
}
