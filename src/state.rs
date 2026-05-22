use super::*;

#[derive(Debug)]
pub(crate) struct State {
  composer: Composer,
  should_quit: bool,
  transcript: Transcript,
}

impl State {
  pub(crate) fn composer(&self) -> &Composer {
    &self.composer
  }

  pub(crate) fn composer_height(&self) -> u16 {
    self.composer.height()
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    match action {
      Action::Backspace => self.composer.backspace(),
      Action::CompleteCommand => self.composer.complete_command(),
      Action::Input(c) => self.composer.push(c),
      Action::Quit => self.quit(),
      Action::SelectNextCommand => self.select_next_command(),
      Action::SelectPreviousCommand => self.select_previous_command(),
      Action::Submit => return self.submit(),
    }

    Vec::new()
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::AgentDelta(delta) => self.transcript.push_agent_delta(&delta),
      Event::AgentDone => self.transcript.finish_agent_message(),
      Event::Error(error) => self.transcript.error(error),
    }

    Vec::new()
  }

  pub(crate) fn input_text(&self) -> &str {
    self.composer.input_text()
  }

  pub(crate) fn new(input: String) -> Self {
    Self {
      composer: Composer::new(input),
      should_quit: false,
      transcript: Transcript::new(),
    }
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn run_command(&mut self, command: Command) {
    match command {
      Command::Clear => self.transcript.clear(),
      Command::Quit => self.quit(),
    }

    self.reset_input();
  }

  fn select_next_command(&mut self) {
    self.composer.select_next_command();
  }

  fn select_previous_command(&mut self) {
    self.composer.select_previous_command();
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self) -> Vec<Effect> {
    let input = self.composer.input_text().trim();

    if let Some(command) = Command::from_input(input) {
      self.run_command(command);
      return Vec::new();
    }

    if input.starts_with('/') {
      let command = self.composer.selected_command();

      if let Some(command) = command {
        self.run_command(command);
      } else if input.len() > 1 {
        self.transcript.push_agent(format!(
          "Unrecognized command '{input}'. Type \"/\" for a list of supported commands."
        ));

        self.reset_input();
      }

      return Vec::new();
    }

    if self.transcript.is_agent_active() {
      return Vec::new();
    }

    if input.is_empty() {
      return Vec::new();
    }

    let input = input.to_string();

    self.transcript.send(input.clone());

    self.reset_input();

    vec![Effect::RunAgent { input }]
  }

  pub(crate) fn transcript(&self) -> &Transcript {
    &self.transcript
  }

  pub(crate) fn transcript_height(&self, width: u16) -> u16 {
    self.transcript.height(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn command_autocomplete() {
    let mut state = State::new("/".into());

    assert_eq!(
      state
        .composer()
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectNextCommand));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.input_text(), "/quit");

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.should_quit());
  }

  #[test]
  fn command_clear() {
    #[track_caller]
    fn case(command: &str) {
      let mut state = State::new("foo".into());

      assert_eq!(
        state.handle_event(Event::Action(Action::Submit)),
        vec![Effect::RunAgent {
          input: "foo".into()
        }]
      );

      state.handle_event(Event::AgentDelta("bar".into()));
      state.handle_event(Event::AgentDone);

      for c in command.chars() {
        state.handle_event(Event::Action(Action::Input(c)));
      }

      state.handle_event(Event::Action(Action::Submit));

      assert_eq!(state.transcript().messages(), &[]);
      assert_eq!(state.input_text(), "");
    }

    case("/");
    case("/c");
  }

  #[test]
  fn error_clears_active_message() {
    let mut state = State::new("foo".into());

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        input: "foo".into()
      }]
    );

    state.handle_event(Event::Error("bar".into()));

    assert_eq!(
      state.transcript().messages()[1],
      Message::new(Role::Agent, "bar")
    );

    for c in "baz".chars() {
      state.handle_event(Event::Action(Action::Input(c)));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        input: "baz".into()
      }]
    );
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new("/foobar".into());

    state.handle_event(Event::Action(Action::Submit));

    assert_eq!(
      state.transcript().messages(),
      &[Message::new(
        Role::Agent,
        "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
      )]
    );

    assert_eq!(state.input_text(), "");
  }
}
