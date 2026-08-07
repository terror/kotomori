use super::*;

#[derive(Debug)]
pub(crate) struct ResumePicker {
  pub(crate) query: String,
  pub(crate) selected: usize,
  sessions: Vec<Session>,
}

impl ResumePicker {
  fn clamp_selection(&mut self) {
    let len = self.filtered_len();

    self.selected = if len == 0 {
      0
    } else {
      self.selected.min(len.saturating_sub(1))
    };
  }

  pub(crate) fn filtered(&self) -> Vec<&Session> {
    self
      .sessions
      .iter()
      .filter(|session| session.matches(&self.query))
      .collect()
  }

  fn filtered_len(&self) -> usize {
    self
      .sessions
      .iter()
      .filter(|session| session.matches(&self.query))
      .count()
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
        key: Key::Char(c),
        ctrl: false,
        alt: false,
        ..
      }) => {
        self.query.push(c);
        self.clamp_selection();
      }
      Action::SelectNext => {
        let len = self.filtered_len();

        if len > 0 {
          self.selected = self.selected.saturating_add(1) % len;
        }
      }
      Action::Submit => {
        if let Some(id) = self.selected_id() {
          return Some(ResumePickerAction::Resume(id));
        }
      }
      Action::Interrupt | Action::Quit => {
        return Some(ResumePickerAction::Cancel);
      }
      Action::SelectPrevious => {
        let len = self.filtered_len();

        if len > 0 {
          self.selected = if self.selected == 0 {
            len.saturating_sub(1)
          } else {
            self.selected.saturating_sub(1)
          };
        }
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

  fn selected_id(&self) -> Option<String> {
    self
      .filtered()
      .get(self.selected)
      .map(|session| session.id.clone())
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
        cwd: "foo".into(),
        entries: Vec::new(),
        id: "foo".into(),
        model: "mock:local".into(),
        persisted: true,
        title: Some("foo".into()),
        updated_at: 0,
      },
      Session {
        created_at: 0,
        cwd: "bar".into(),
        entries: Vec::new(),
        id: "bar".into(),
        model: "mock:local".into(),
        persisted: true,
        title: Some("bar".into()),
        updated_at: 0,
      },
    ]);

    picker.handle_action(Action::Edit(Input {
      key: Key::Char('b'),
      ..Default::default()
    }));

    assert_eq!(
      picker
        .filtered()
        .into_iter()
        .map(|session| session.id.as_str())
        .collect::<Vec<_>>(),
      ["bar"],
    );
  }
}
