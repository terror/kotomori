use super::*;

#[derive(Debug)]
pub(crate) struct ResumePicker {
  pub(crate) query: String,
  pub(crate) scroll: ScrollState,
  pub(crate) selected: usize,
  sessions: Vec<SessionSummary>,
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

  pub(crate) fn filtered(&self) -> Vec<&SessionSummary> {
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
        alt: false,
        ctrl: false,
        key: Key::Char(c),
        ..
      }) => {
        self.query.push(c);
        self.clamp_selection();
      }
      Action::Paste(input) => {
        self.query.push_str(&input);
        self.clamp_selection();
      }
      Action::SelectNext => {
        let len = self.filtered_len();

        if len > 0 {
          self.selected = self.selected.saturating_add(1) % len;
        }
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
        let len = self.filtered_len();

        if len > 0 {
          self.selected = if self.selected == 0 {
            len.saturating_sub(1)
          } else {
            self.selected.saturating_sub(1)
          };
        }
      }
      Action::CompleteCommand | Action::Edit(_) | Action::ToggleReasoning => {}
    }

    None
  }

  pub(crate) fn new(sessions: Vec<SessionSummary>) -> Self {
    Self {
      query: String::new(),
      scroll: ScrollState::default(),
      selected: 0,
      sessions,
    }
  }

  fn selected_id(&self) -> Option<i64> {
    self.filtered().get(self.selected).map(|session| session.id)
  }
}
