use super::*;

#[derive(Debug)]
pub(crate) struct Transcript {
  agent_message: Option<usize>,
  messages: Vec<Message>,
}

impl Transcript {
  pub(crate) fn clear(&mut self) {
    self.agent_message = None;
    self.messages.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    if let Some(index) = self.agent_message.take() {
      self.messages[index].content = error;
    } else {
      self.messages.push(Message::new(Role::Agent, error));
    }
  }

  pub(crate) fn finish_agent_message(&mut self) {
    self.agent_message = None;
  }

  pub(crate) fn height(&self, width: u16) -> u16 {
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

  pub(crate) fn is_agent_active(&self) -> bool {
    self.agent_message.is_some()
  }

  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn new() -> Self {
    Self {
      agent_message: None,
      messages: Vec::new(),
    }
  }

  pub(crate) fn push_agent(&mut self, content: impl Into<String>) {
    self.messages.push(Message::new(Role::Agent, content));
  }

  pub(crate) fn push_agent_delta(&mut self, delta: &str) {
    if let Some(index) = self.agent_message {
      self.messages[index].content.push_str(delta);
    }
  }

  pub(crate) fn send(&mut self, input: String) {
    self.messages.push(Message::new(Role::User, input));
    self.messages.push(Message::new(Role::Agent, ""));

    self.agent_message = self.messages.len().checked_sub(1);
  }
}

impl Widget for &Transcript {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let lines = self
      .messages()
      .iter()
      .flat_map(Message::lines)
      .collect::<Vec<_>>();

    Paragraph::new(lines)
      .wrap(Wrap { trim: false })
      .render(area, buf);
  }
}
