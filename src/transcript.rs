use super::*;

#[derive(Debug)]
pub(crate) struct Transcript {
  active_agent_message: Option<String>,
  active_frame: usize,
  messages: Vec<Message>,
  tool_invocations: BTreeMap<String, ToolInvocation>,
  tool_results: BTreeMap<String, ToolResult>,
}

impl Transcript {
  const FRAMES: &[&str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  pub(crate) fn clear(&mut self) {
    self.active_agent_message = None;
    self.messages.clear();
    self.tool_invocations.clear();
    self.tool_results.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.active_agent_message = None;
    self.messages.push(Message::new(Role::Agent, error));
  }

  pub(crate) fn finish_agent_message(&mut self) {
    if let Some(message) = self.active_agent_message.take()
      && !message.is_empty()
    {
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
      tool_invocations: BTreeMap::new(),
      tool_results: BTreeMap::new(),
    }
  }

  pub(crate) fn push_agent(&mut self, content: impl Into<String>) {
    self.messages.push(Message::new(Role::Agent, content));
  }

  pub(crate) fn push_agent_delta(&mut self, delta: &str) {
    if let Some(message) = &mut self.active_agent_message {
      message.push_str(delta);
    } else {
      self.active_agent_message = Some(delta.into());
    }
  }

  pub(crate) fn push_tool_call(&mut self, invocation: ToolInvocation) {
    self.finish_agent_message();

    self.messages.push(invocation.message());

    self
      .tool_invocations
      .insert(invocation.id.clone(), invocation);

    self.active_agent_message = Some(String::new());
  }

  pub(crate) fn push_tool_result(&mut self, id: String, result: ToolResult) {
    self.messages.push(Message::tool_result(
      id.clone(),
      result.message_content(),
      result.is_error(),
    ));

    self.tool_results.insert(id, result);

    self.active_agent_message = Some(String::new());
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
    let mut lines = Vec::new();

    for message in self.messages() {
      match message.kind() {
        MessageKind::Text {
          content,
          role: Role::Agent,
        } => {
          lines.extend(
            once(Line::blank())
              .chain(content.lines().map(|line| Line::raw(format!(" {line}"))))
              .chain(once(Line::blank())),
          );
        }
        MessageKind::Text {
          content: _,
          role: Role::User,
        } => lines.extend(message.render(width)),
        MessageKind::ToolUse { id, .. } => {
          let Some(invocation) = self.tool_invocations.get(id) else {
            continue;
          };

          let transcript_tool_invocation = TranscriptToolInvocation::new(
            invocation,
            self.tool_results.get(id),
          );

          lines.extend(transcript_tool_invocation.render(width));
        }
        MessageKind::ToolResult { .. } => {}
      }
    }

    match self.active_agent_message.as_deref() {
      Some("") => {
        if !lines.last().is_some_and(|line| line == &Line::blank()) {
          lines.push(Line::blank());
        }

        lines.extend([
          vec![
            Span::styled(Self::spinner(self.active_frame), Style::CyanBold),
            Span::styled(" Working...", Style::Gray),
          ]
          .into(),
          Line::blank(),
        ]);
      }
      Some(message) => {
        if !lines.last().is_some_and(|line| line == &Line::blank()) {
          lines.push(Line::blank());
        }

        lines.extend(
          message
            .lines()
            .map(|line| Line::raw(format!(" {line}")))
            .chain(once(Line::blank())),
        );
      }
      None => {}
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

    assert!(
      transcript.render(80).ends_with(&[
        Line::blank(),
        vec![
          Span::styled("✶", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
        ]
        .into(),
        Line::blank(),
      ])
    );

    transcript.push_agent_delta("bar");

    assert!(transcript.render(80).ends_with(&[
      Line::blank(),
      Line::raw(" bar"),
      Line::blank(),
    ]));
  }
}
