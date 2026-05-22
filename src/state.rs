use super::*;

#[derive(Debug)]
pub(crate) struct State {
  agent_message: Option<usize>,
  command_index: usize,
  input: String,
  messages: Vec<Message>,
  should_quit: bool,
}

impl State {
  fn backspace(&mut self) {
    self.input.pop();
    self.command_index = 0;
  }

  fn clear_transcript(&mut self) {
    self.agent_message = None;
    self.messages.clear();
  }

  pub(crate) fn command_height(&self) -> u16 {
    u16::try_from(self.commands().count()).unwrap_or(u16::MAX)
  }

  pub(crate) fn commands(&self) -> impl Iterator<Item = Command> + '_ {
    Command::iter().filter(|command| command.matches(&self.input))
  }

  fn complete_command(&mut self) {
    if let Some(command) = self.selected_command() {
      self.input = command.input();
    }
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    match action {
      Action::Backspace => self.backspace(),
      Action::CompleteCommand => self.complete_command(),
      Action::Input(c) => self.input(c),
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
      Event::AgentDelta(delta) => {
        if let Some(index) = self.agent_message {
          self.messages[index].content.push_str(&delta);
        }
      }
      Event::AgentDone => self.agent_message = None,
      Event::Error(error) => {
        if let Some(index) = self.agent_message.take() {
          self.messages[index].content = error;
        } else {
          self.messages.push(Message::new(Role::Agent, error));
        }
      }
    }

    Vec::new()
  }

  fn input(&mut self, c: char) {
    self.input.push(c);
    self.command_index = 0;
  }

  pub(crate) fn input_text(&self) -> &str {
    &self.input
  }

  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn new(input: String) -> Self {
    Self {
      agent_message: None,
      command_index: 0,
      input,
      messages: Vec::new(),
      should_quit: false,
    }
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.input.clear();
    self.command_index = 0;
  }

  fn run_command(&mut self, command: Command) {
    match command {
      Command::Clear => self.clear_transcript(),
      Command::Quit => self.quit(),
    }

    self.reset_input();
  }

  fn select_next_command(&mut self) {
    let len = self.commands().count();

    if len > 0 {
      self.command_index = self.command_index.saturating_add(1) % len;
    }
  }

  fn select_previous_command(&mut self) {
    let len = self.commands().count();

    if len > 0 {
      self.command_index = if self.command_index == 0 {
        len.saturating_sub(1)
      } else {
        self.command_index.saturating_sub(1)
      };
    }
  }

  fn selected_command(&self) -> Option<Command> {
    self.commands().nth(self.selected_command_index()?)
  }

  pub(crate) fn selected_command_index(&self) -> Option<usize> {
    let len = self.commands().count();

    if len == 0 {
      None
    } else {
      Some(self.command_index.min(len.saturating_sub(1)))
    }
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self) -> Vec<Effect> {
    let input = self.input.trim();

    if let Some(command) = Command::from_input(input) {
      self.run_command(command);

      return Vec::new();
    }

    if input.starts_with('/') {
      if input.len() > 1 {
        let command = self.selected_command();

        if let Some(command) = command {
          self.run_command(command);
        } else {
          self.messages.push(Message::new(
            Role::Agent,
            format!(
              "Unrecognized command '{input}'. Type \"/\" for a list of supported commands."
            ),
          ));

          self.reset_input();
        }
      }

      return Vec::new();
    }

    if self.agent_message.is_some() {
      return Vec::new();
    }

    if input.is_empty() {
      return Vec::new();
    }

    let input = input.to_string();

    self.messages.push(Message::new(Role::User, input.clone()));
    self.messages.push(Message::new(Role::Agent, ""));

    self.agent_message = self.messages.len().checked_sub(1);

    self.input.clear();

    vec![Effect::RunAgent { input }]
  }

  pub(crate) fn transcript_height(&self, width: u16) -> u16 {
    let width = usize::from(width.max(1));

    self
      .messages
      .iter()
      .map(|message| {
        let len = message.width();

        u16::try_from(len.div_ceil(width).max(1)).unwrap_or(u16::MAX)
      })
      .fold(0u16, u16::saturating_add)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn command_autocomplete() {
    let mut state = State::new("/".into());

    assert_eq!(
      state.commands().map(Command::name).collect::<Vec<_>>(),
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
    let mut state = State::new("foo".into());

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        input: "foo".into()
      }]
    );

    state.handle_event(Event::AgentDelta("bar".into()));
    state.handle_event(Event::AgentDone);

    for c in "/c".chars() {
      state.handle_event(Event::Action(Action::Input(c)));
    }

    state.handle_event(Event::Action(Action::Submit));

    assert_eq!(state.messages(), &[]);
    assert_eq!(state.input_text(), "");
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

    assert_eq!(state.messages()[1], Message::new(Role::Agent, "bar"));

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
      state.messages(),
      &[Message::new(
        Role::Agent,
        "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
      )]
    );

    assert_eq!(state.input_text(), "");
  }
}
