use super::*;

#[derive(Debug)]
pub(crate) enum Screen {
  Quit,
  Resume(ResumePicker),
  Session(Box<State>),
}

impl Screen {
  pub(crate) fn agent(
    &self,
    event_sender: UnboundedSender<Event>,
  ) -> Result<Option<Agent>> {
    match self {
      Self::Quit | Self::Resume(_) => Ok(None),
      Self::Session(state) => {
        Agent::new(event_sender, &state.session.settings).map(Some)
      }
    }
  }

  pub(crate) fn should_quit(&self) -> bool {
    match self {
      Self::Quit => true,
      Self::Resume(_) => false,
      Self::Session(state) => state.should_quit,
    }
  }
}
