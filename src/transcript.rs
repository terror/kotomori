use super::*;

#[derive(Debug)]
pub(crate) struct Transcript {
  active_agent_message: Option<String>,
  messages: Vec<Message>,
}

impl Transcript {
  pub(crate) fn clear(&mut self) {
    self.active_agent_message = None;
    self.messages.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.active_agent_message = None;
    self.messages.push(Message::new(Role::Agent, error));
  }

  pub(crate) fn finish_agent_message(&mut self) {
    if let Some(message) = self.active_agent_message.take() {
      self.messages.push(Message::new(Role::Agent, message));
    }
  }

  pub(crate) fn is_agent_active(&self) -> bool {
    self.active_agent_message.is_some()
  }

  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn new() -> Self {
    Self {
      active_agent_message: None,
      messages: Vec::new(),
    }
  }

  pub(crate) fn push_agent(&mut self, content: impl Into<String>) {
    self.messages.push(Message::new(Role::Agent, content));
  }

  pub(crate) fn push_agent_delta(&mut self, delta: &str) {
    if let Some(message) = &mut self.active_agent_message {
      message.push_str(delta);
    }
  }

  pub(crate) fn send(&mut self, input: String) {
    self.messages.push(Message::new(Role::User, input));
    self.active_agent_message = Some(String::new());
  }
}

impl Component for Transcript {
  fn render(&self, _width: u16) -> Vec<Line> {
    let mut lines = self
      .messages()
      .iter()
      .flat_map(Message::lines)
      .collect::<Vec<_>>();

    if let Some(message) = &self.active_agent_message {
      lines.extend(Message::new(Role::Agent, message).lines());
    }

    lines
  }
}
