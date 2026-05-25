use super::*;

#[derive(Debug)]
pub(crate) struct Transcript {
  active_agent_message: Option<String>,
  active_frame: usize,
  messages: Vec<Message>,
}

impl Transcript {
  const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
      active_frame: 0,
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
    self.active_frame = 0;
  }

  fn spinner(frame: usize) -> &'static str {
    Self::FRAMES[frame % Self::FRAMES.len()]
  }

  pub(crate) fn tick(&mut self) {
    if self.is_agent_active() {
      self.active_frame = self.active_frame.wrapping_add(1);
    }
  }
}

impl Component for Transcript {
  fn render(&self, width: u16) -> Vec<Line> {
    let mut lines = self
      .messages()
      .iter()
      .flat_map(|message| message.render(width))
      .collect::<Vec<_>>();

    if let Some(message) = &self.active_agent_message {
      if message.is_empty() {
        lines.push(Line::raw(format!(
          "{} Working...",
          Self::spinner(self.active_frame)
        )));
      } else {
        lines.extend(Message::new(Role::Agent, message).render(width));
      }
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn active_rendering() {
    let mut transcript = Transcript::new();

    transcript.send("foo".into());
    for _ in 0..4 {
      transcript.tick();
    }

    assert_eq!(
      transcript.render(80).last(),
      Some(&Line::raw("⠼ Working..."))
    );

    transcript.push_agent_delta("bar");

    assert_eq!(transcript.render(80).last(), Some(&Line::raw("bar")));
  }
}
