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

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    match action {
      Action::CompleteCommand => {
        if !self.composer.complete_command() {
          self.composer.input(Input {
            key: Key::Tab,
            ..Default::default()
          });
        }
      }
      Action::Edit(input) => self.composer.input(input),
      Action::Quit => self.quit(),
      Action::SelectNextCommand => {
        if self.composer.selected_command().is_some() {
          self.composer.select_next_command();
        } else {
          self.composer.input(Input {
            key: Key::Down,
            ..Default::default()
          });
        }
      }
      Action::SelectPreviousCommand => {
        if self.composer.selected_command().is_some() {
          self.composer.select_previous_command();
        } else {
          self.composer.input(Input {
            key: Key::Up,
            ..Default::default()
          });
        }
      }
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

  #[cfg(test)]
  pub(crate) fn input_text(&self) -> String {
    self.composer.input_text()
  }

  pub(crate) fn new(input: &str) -> Self {
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

  fn run_command(&mut self, command: Command) -> Vec<Effect> {
    match command {
      Command::Clear => self.transcript.clear(),
      Command::Quit => self.quit(),
    }

    self.reset_input();

    Vec::new()
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self) -> Vec<Effect> {
    let input = self.composer.input_text();
    let input = input.trim();

    if let Some(command) = Command::from_input(input) {
      return self.run_command(command);
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
}

#[cfg(test)]
mod tests {
  use super::*;

  fn input(c: char) -> Event {
    Event::Action(Action::Edit(Input {
      key: Key::Char(c),
      ..Default::default()
    }))
  }

  #[test]
  fn command_autocomplete() {
    let mut state = State::new("/");

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
      let mut state = State::new("foo");

      assert_eq!(
        state.handle_event(Event::Action(Action::Submit)),
        vec![Effect::RunAgent {
          input: "foo".into()
        }]
      );

      state.handle_event(Event::AgentDelta("bar".into()));
      state.handle_event(Event::AgentDone);

      for c in command.chars() {
        state.handle_event(input(c));
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
    let mut state = State::new("foo");

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
      state.handle_event(input(c));
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
    let mut state = State::new("/foobar");

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

  #[test]
  fn multiline_input() {
    let mut state = State::new("");

    for c in "foo".chars() {
      state.handle_event(input(c));
    }

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Enter,
      ..Default::default()
    })));

    for c in "bar".chars() {
      state.handle_event(input(c));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        input: "foo\nbar".into()
      }]
    );
  }
}
