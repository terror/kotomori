use super::*;

#[derive(Debug, Default)]
pub(crate) struct Transcript {
  active_agent_message: Option<String>,
  active_frame: usize,
  entries: Vec<TranscriptEntry>,
}

impl Transcript {
  const FRAMES: &[&str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  pub(crate) fn clear(&mut self) {
    self.active_agent_message = None;
    self.entries.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.active_agent_message = None;
    self.entries.push(TranscriptEntry::Agent(error));
  }

  fn find_tool_result_mut(
    &mut self,
    id: &str,
  ) -> Option<&mut Option<ToolResult>> {
    self.entries.iter_mut().rev().find_map(|entry| match entry {
      TranscriptEntry::Tool { invocation, result } if invocation.id == id => {
        Some(result)
      }
      _ => None,
    })
  }

  pub(crate) fn finish_agent_message(&mut self) {
    let Some(message) = self.active_agent_message.take() else {
      return;
    };

    if message.is_empty() {
      return;
    }

    self.entries.push(TranscriptEntry::Agent(message));
  }

  pub(crate) fn is_agent_active(&self) -> bool {
    self.active_agent_message.is_some()
  }

  pub(crate) fn messages(&self) -> Vec<Message> {
    self
      .entries
      .iter()
      .flat_map(TranscriptEntry::messages)
      .collect()
  }

  pub(crate) fn push_agent(&mut self, content: impl Into<String>) {
    self.entries.push(TranscriptEntry::Agent(content.into()));
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

    self.entries.push(TranscriptEntry::Tool {
      invocation,
      result: None,
    });

    self.active_agent_message = Some(String::new());
  }

  pub(crate) fn push_tool_result(&mut self, id: &str, result: ToolResult) {
    if let Some(entry_result) = self.find_tool_result_mut(id) {
      *entry_result = Some(result);
    }

    self.active_agent_message = Some(String::new());
  }

  pub(crate) fn send(&mut self, input: String) {
    self.entries.push(TranscriptEntry::User(input));
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

    for entry in &self.entries {
      match entry {
        TranscriptEntry::Agent(content) => {
          lines.extend(
            once(Line::blank())
              .chain(content.lines().map(|line| Line::raw(format!(" {line}"))))
              .chain(once(Line::blank())),
          );
        }
        TranscriptEntry::Tool { invocation, result } => {
          lines.extend(
            TranscriptToolInvocation::new(invocation, result.as_ref())
              .render(width),
          );
        }
        TranscriptEntry::User(content) => {
          lines.extend(Message::new(Role::User, content.clone()).render(width));
        }
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
    let mut transcript = Transcript::default();

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

  #[test]
  fn tool_messages() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }),
    };

    transcript.push_tool_call(invocation.clone());

    assert_eq!(transcript.messages(), vec![invocation.message()]);

    let result = ToolResult::command(Some(0), "bar\n", "");

    transcript.push_tool_result("foo", result.clone());

    assert_eq!(
      transcript.messages(),
      vec![invocation.message(), result.message("foo")]
    );
  }
}
