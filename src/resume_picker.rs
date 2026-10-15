use super::*;

#[derive(Debug)]
pub(crate) struct ResumePicker {
  pub(crate) query: String,
  pub(crate) scroll: Scroll,
  pub(crate) selection: Selection,
  pub(crate) sessions: Vec<SessionSummary>,
}

impl ResumePicker {
  fn clamp_selection(&mut self) {
    self.selection.clamp(self.filtered_len());
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
        self.selection.select_next(self.filtered_len());
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
        self.selection.select_previous(self.filtered_len());
      }
      Action::CompleteCommand | Action::Edit(_) | Action::ToggleReasoning => {}
    }

    None
  }

  pub(crate) fn new(sessions: Vec<SessionSummary>) -> Self {
    Self {
      query: String::new(),
      scroll: Scroll::default(),
      selection: Selection::default(),
      sessions,
    }
  }

  fn selected_id(&self) -> Option<i64> {
    let filtered = self.filtered();

    filtered
      .get(self.selection.index(filtered.len())?)
      .map(|session| session.id)
  }
}
