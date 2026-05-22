use super::*;

#[derive(Debug)]
pub(crate) struct State {
  agent_message: Option<usize>,
  input: String,
  messages: Vec<Message>,
  should_quit: bool,
}

impl State {
  fn handle_action(&mut self, action: Action) -> Option<Effect> {
    match action {
      Action::Backspace => {
        self.input.pop();
      }
      Action::Input(c) => self.input.push(c),
      Action::Quit => self.should_quit = true,
      Action::Submit => return self.submit(),
    }

    None
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Option<Effect> {
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

    None
  }

  pub(crate) fn input(&self) -> &str {
    &self.input
  }

  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn new(input: String) -> Self {
    Self {
      agent_message: None,
      input,
      messages: Vec::new(),
      should_quit: false,
    }
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self) -> Option<Effect> {
    if self.agent_message.is_some() {
      return None;
    }

    let input = self.input.trim();

    if input.is_empty() {
      return None;
    }

    let input = input.to_string();

    self.messages.push(Message::new(Role::User, input.clone()));
    self.messages.push(Message::new(Role::Agent, ""));

    self.agent_message = self.messages.len().checked_sub(1);

    self.input.clear();

    Some(Effect::RunAgent { input })
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
  fn error_clears_active_message() {
    let mut state = State::new("foo".into());

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Some(Effect::RunAgent {
        input: "foo".into()
      })
    );

    state.handle_event(Event::Error("bar".into()));

    assert_eq!(state.messages()[1], Message::new(Role::Agent, "bar"));

    for c in "baz".chars() {
      state.handle_event(Event::Action(Action::Input(c)));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Some(Effect::RunAgent {
        input: "baz".into()
      })
    );
  }
}
