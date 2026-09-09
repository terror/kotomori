use super::*;

#[derive(Debug)]
pub(crate) struct ResumePicker {
  pub(crate) query: String,
  pub(crate) selected: usize,
  sessions: Vec<Session>,
}

impl ResumePicker {
  fn clamp_selection(&mut self) {
    self.selected =
      self.selected.min(self.filtered().count().saturating_sub(1));
  }

  pub(crate) fn filtered(&self) -> impl Iterator<Item = &Session> + '_ {
    self
      .sessions
      .iter()
      .filter(|session| session.matches(&self.query))
  }

  pub(crate) fn handle_action(
    &mut self,
    action: Action,
  ) -> Option<ResumePickerAction> {
    match action {
      Action::Edit(input) if input.key == Key::Backspace => {
        self.query.pop();
        self.clamp_selection();
      }
      Action::Edit(input) if input.key == Key::Char('u') && input.ctrl => {
        self.query.clear();
        self.clamp_selection();
      }
      Action::Edit(Input {
        alt: false,
        ctrl: false,
        key: Key::Char(c),
        ..
      }) => {
        self.query.push(c);
        self.clamp_selection();
      }
      Action::SelectNext => {
        self.selected = selection::next(self.selected, self.filtered().count());
      }
      Action::Submit | Action::SubmitImmediately => {
        if let Some(id) = self.selected_id() {
          return Some(ResumePickerAction::Resume(id));
        }
      }
      Action::Interrupt | Action::Quit => {
        return Some(ResumePickerAction::Cancel);
      }
      Action::SelectPrevious => {
        self.selected =
          selection::previous(self.selected, self.filtered().count());
      }
      Action::CompleteCommand | Action::Edit(_) => {}
    }

    None
  }

  pub(crate) fn new(sessions: Vec<Session>) -> Self {
    Self {
      query: String::new(),
      selected: 0,
      sessions,
    }
  }

  fn selected_id(&self) -> Option<i64> {
    self
      .filtered()
      .nth(self.selected)
      .and_then(|session| session.id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn filters_sessions() {
    let mut picker = ResumePicker::new(vec![
      Session {
        created_at: 0,
        directory: "foo".into(),
        id: Some(1),
        model: "mock:local".into(),
        title: Some("foo".into()),
        transcript: Transcript::default(),
        updated_at: 0,
      },
      Session {
        created_at: 0,
        directory: "bar".into(),
        id: Some(2),
        model: "mock:local".into(),
        title: Some("bar".into()),
        transcript: Transcript::default(),
        updated_at: 0,
      },
    ]);

    picker.handle_action(Action::SelectNext);

    assert_eq!(picker.selected_id(), Some(2));

    picker.handle_action(Action::Edit(Input {
      key: Key::Char('b'),
      ..Default::default()
    }));

    assert_eq!(
      picker
        .filtered()
        .filter_map(|session| session.id)
        .collect::<Vec<_>>(),
      [2],
    );
    assert_eq!(picker.selected, 0);
    assert_eq!(picker.selected_id(), Some(2));

    picker.handle_action(Action::Edit(Input {
      key: Key::Char('x'),
      ..Default::default()
    }));

    assert_eq!(picker.filtered().count(), 0);
    assert_eq!(picker.selected, 0);
    assert_eq!(picker.selected_id(), None);

    picker.handle_action(Action::Edit(Input {
      key: Key::Char('u'),
      ctrl: true,
      ..Default::default()
    }));
    picker.handle_action(Action::SelectPrevious);

    assert_eq!(picker.selected_id(), Some(2));
  }
}
