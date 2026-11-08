use super::*;

#[derive(Debug)]
pub(crate) enum Screen {
  Quit,
  Resume(ResumePicker),
  Session(Box<State>),
}

impl Screen {
  pub(crate) fn should_quit(&self) -> bool {
    match self {
      Self::Quit => true,
      Self::Resume(_) => false,
      Self::Session(state) => state.should_quit,
    }
  }
}
