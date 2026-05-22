use super::*;

#[derive(Debug)]
pub(crate) struct State {
  agent_message: Option<usize>,
  input: String,
  messages: Vec<Message>,
  should_quit: bool,
}

impl State {
  pub(crate) fn handle_action(&mut self, action: Action) -> Option<Effect> {
    match action {
      Action::AgentDone => self.agent_message = None,
      Action::AgentOutput(c) => {
        if let Some(index) = self.agent_message {
          self.messages[index].content.push(c);
        }
      }
      Action::Backspace => {
        self.input.pop();
      }
      Action::Input(c) => self.input.push(c),
      Action::Quit => self.should_quit = true,
      Action::Submit => return self.submit(),
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
