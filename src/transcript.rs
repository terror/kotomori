use super::*;

#[derive(Debug, Default)]
pub(crate) struct Transcript {
  active_agent_activity: AgentActivity,
  active_elapsed: Duration,
  active_frame: usize,
  pub(crate) entries: Vec<TranscriptEntry>,
}

impl Transcript {
  const FRAMES: &[&str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  pub(crate) fn clear(&mut self) {
    self.active_agent_activity = AgentActivity::Idle;
    self.active_elapsed = Duration::ZERO;
    self.entries.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.active_agent_activity = AgentActivity::Idle;
    self.active_elapsed = Duration::ZERO;
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

  pub(crate) fn finish_agent_activity(&mut self) {
    match mem::take(&mut self.active_agent_activity) {
      AgentActivity::Reasoning(reasoning) if !reasoning.is_empty() => {
        self.entries.push(TranscriptEntry::Reasoning(reasoning));
      }
      AgentActivity::Streaming(message) if !message.is_empty() => {
        self.entries.push(TranscriptEntry::Agent(message));
      }
      AgentActivity::Idle
      | AgentActivity::Reasoning(_)
      | AgentActivity::Streaming(_)
      | AgentActivity::Waiting => {}
    }
  }

  pub(crate) fn interrupt(&mut self) {
    self.finish_agent_activity();
    self.active_elapsed = Duration::ZERO;
    self.entries.push(TranscriptEntry::Interrupted);
  }

  pub(crate) fn is_agent_active(&self) -> bool {
    !matches!(self.active_agent_activity, AgentActivity::Idle)
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.entries.is_empty() && !self.is_agent_active()
  }

  pub(crate) fn messages(&self) -> Vec<Message> {
    let mut messages = Vec::new();

    let (mut agent_content, mut tool_results) = (Vec::new(), Vec::new());

    for (index, entry) in self.entries.iter().enumerate() {
      match entry {
        TranscriptEntry::Agent(content) => {
          agent_content.push(AgentMessageContent::Text(content.clone()));
        }
        TranscriptEntry::Interrupted => {}
        TranscriptEntry::Reasoning(reasoning) => {
          agent_content.push(AgentMessageContent::Reasoning(reasoning.clone()));
        }
        TranscriptEntry::Tool { invocation, result } => {
          let next_is_tool = matches!(
            self.entries.get(index + 1),
            Some(TranscriptEntry::Tool { .. })
          );

          agent_content.push(AgentMessageContent::ToolCall(invocation.clone()));

          if let Some(result) = result {
            tool_results.push(result.message(invocation.id.clone()));
          }

          if !next_is_tool {
            if !agent_content.is_empty() {
              messages.push(Message::Agent(mem::take(&mut agent_content)));
            }

            messages.append(&mut tool_results);
          }
        }
        TranscriptEntry::User(content) => {
          if !agent_content.is_empty() {
            messages.push(Message::Agent(mem::take(&mut agent_content)));
          }

          messages.append(&mut tool_results);

          messages.push(Message::User(vec![UserMessageContent::Text(
            content.clone(),
          )]));
        }
      }
    }

    if !agent_content.is_empty() {
      messages.push(Message::Agent(agent_content));
    }

    messages.append(&mut tool_results);

    messages
  }

  pub(crate) fn push_agent(&mut self, content: impl Into<String>) {
    self.entries.push(TranscriptEntry::Agent(content.into()));
  }

  pub(crate) fn push_agent_delta(&mut self, delta: &str) {
    self.active_agent_activity =
      match mem::take(&mut self.active_agent_activity) {
        AgentActivity::Idle | AgentActivity::Waiting if delta.is_empty() => {
          AgentActivity::Waiting
        }
        AgentActivity::Idle | AgentActivity::Waiting => {
          AgentActivity::Streaming(delta.into())
        }
        AgentActivity::Reasoning(reasoning) => {
          if !reasoning.is_empty() {
            self.entries.push(TranscriptEntry::Reasoning(reasoning));
          }

          if delta.is_empty() {
            AgentActivity::Waiting
          } else {
            AgentActivity::Streaming(delta.into())
          }
        }
        AgentActivity::Streaming(mut message) => {
          message.push_str(delta);
          AgentActivity::Streaming(message)
        }
      };
  }

  pub(crate) fn push_agent_reasoning_delta(&mut self, delta: &str) {
    self.active_agent_activity =
      match mem::take(&mut self.active_agent_activity) {
        AgentActivity::Idle | AgentActivity::Waiting if delta.is_empty() => {
          AgentActivity::Waiting
        }
        AgentActivity::Idle | AgentActivity::Waiting => {
          AgentActivity::Reasoning(delta.into())
        }
        AgentActivity::Reasoning(mut reasoning) => {
          reasoning.push_str(delta);
          AgentActivity::Reasoning(reasoning)
        }
        AgentActivity::Streaming(message) => {
          if !message.is_empty() {
            self.entries.push(TranscriptEntry::Agent(message));
          }

          if delta.is_empty() {
            AgentActivity::Waiting
          } else {
            AgentActivity::Reasoning(delta.into())
          }
        }
      };
  }

  pub(crate) fn push_tool_call(&mut self, invocation: ToolInvocation) {
    self.finish_agent_activity();

    self.entries.push(TranscriptEntry::Tool {
      invocation,
      result: None,
    });

    self.active_agent_activity = AgentActivity::Waiting;
  }

  pub(crate) fn push_tool_result(&mut self, id: &str, result: ToolResult) {
    if let Some(entry_result) = self.find_tool_result_mut(id) {
      *entry_result = Some(result);
    }

    self.active_agent_activity = AgentActivity::Waiting;
  }

  pub(crate) fn send(&mut self, input: String) {
    self.active_agent_activity = AgentActivity::Waiting;
    self.active_elapsed = Duration::ZERO;
    self.active_frame = 0;
    self.entries.push(TranscriptEntry::User(input));
  }

  fn spinner(frame: usize) -> &'static str {
    Self::FRAMES[frame % Self::FRAMES.len()]
  }

  pub(crate) fn tick(&mut self, elapsed: Duration) {
    if self.is_agent_active() {
      self.active_elapsed = self.active_elapsed.saturating_add(elapsed);
      self.active_frame = self.active_frame.wrapping_add(1);
    }
  }

  pub(crate) fn with_entries(entries: Vec<TranscriptEntry>) -> Self {
    Self {
      entries,
      ..Self::default()
    }
  }
}

impl Component for Transcript {
  #[allow(clippy::too_many_lines)]
  fn render(&self, width: u16) -> Vec<Line> {
    let mut lines = Vec::new();

    for entry in &self.entries {
      match entry {
        TranscriptEntry::Agent(content) => {
          if !lines.last().is_some_and(|line| line == &Line::blank()) {
            lines.push(Line::blank());
          }

          lines.extend(
            content
              .lines()
              .map(|line| Line::raw(format!(" {line}")))
              .chain(once(Line::blank())),
          );
        }
        TranscriptEntry::Interrupted => {
          lines.extend([
            Line::blank(),
            Line::from([Span::styled(
              "■ Conversation interrupted, tell the model what to do differently.",
              Style::RedBold,
            )]),
            Line::blank(),
          ]);
        }
        TranscriptEntry::Reasoning(reasoning) => {
          if !lines.last().is_some_and(|line| line == &Line::blank()) {
            lines.push(Line::blank());
          }

          lines.extend(reasoning.lines().map(|line| {
            Line::from([Span::styled(format!(" {line}"), Style::DarkGray)])
          }));

          lines.push(Line::blank());
        }
        TranscriptEntry::Tool { invocation, result } => {
          if !lines.last().is_some_and(|line| line == &Line::blank()) {
            lines.push(Line::blank());
          }

          lines.extend(
            TranscriptToolInvocation::new(invocation, result.as_ref())
              .render(width),
          );
        }
        TranscriptEntry::User(content) => {
          lines.extend(
            Message::User(vec![UserMessageContent::Text(content.clone())])
              .render(width),
          );
        }
      }
    }

    match &self.active_agent_activity {
      AgentActivity::Idle => {}
      AgentActivity::Reasoning(reasoning) => {
        if !lines.last().is_some_and(|line| line == &Line::blank()) {
          lines.push(Line::blank());
        }

        lines.extend(reasoning.lines().map(|line| {
          Line::from([Span::styled(format!(" {line}"), Style::DarkGray)])
        }));

        lines.push(Line::blank());

        lines.push(Line::from([
          Span::styled(Self::spinner(self.active_frame), Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(
            format!(" ({} • esc to interrupt)", self.active_elapsed.format()),
            Style::DarkGray,
          ),
        ]));

        lines.push(Line::blank());
      }
      AgentActivity::Streaming(message) => {
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
      AgentActivity::Waiting => {
        if !lines.last().is_some_and(|line| line == &Line::blank()) {
          lines.push(Line::blank());
        }

        lines.extend([
          Line::from([
            Span::styled(Self::spinner(self.active_frame), Style::CyanBold),
            Span::styled(" Working...", Style::Gray),
            Span::styled(
              format!(" ({} • esc to interrupt)", self.active_elapsed.format()),
              Style::DarkGray,
            ),
          ]),
          Line::blank(),
        ]);
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
    let mut transcript = Transcript::default();

    transcript.send("foo".into());

    for elapsed in [
      Duration::from_secs(30),
      Duration::from_secs(30),
      Duration::from_secs(30),
      Duration::from_secs(21),
    ] {
      transcript.tick(elapsed);
    }

    assert!(transcript.render(80).ends_with(&[
      Line::blank(),
      Line::from([
        Span::styled("✶", Style::CyanBold),
        Span::styled(" Working...", Style::Gray),
        Span::styled(" (1m 51s • esc to interrupt)", Style::DarkGray),
      ]),
      Line::blank(),
    ]));

    transcript.push_agent_delta("bar");

    assert!(transcript.render(80).ends_with(&[
      Line::blank(),
      Line::raw(" bar"),
      Line::blank(),
    ]));
  }

  #[test]
  fn interrupted_rendering() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.interrupt();

    assert!(transcript.render(80).ends_with(&[
      Line::blank(),
      Line::from([Span::styled(
        "■ Conversation interrupted, tell the model what to do differently.",
        Style::RedBold,
      )]),
      Line::blank(),
    ]));

    assert_eq!(
      transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[test]
  fn reasoning_rendering() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.tick(Duration::from_millis(120));
    transcript.push_agent_reasoning_delta("bar");

    assert!(transcript.render(80).ends_with(&[
      Line::blank(),
      Line::from([Span::styled(" bar", Style::DarkGray)]),
      Line::blank(),
      Line::from([
        Span::styled("✧", Style::CyanBold),
        Span::styled(" Working...", Style::Gray),
        Span::styled(" (0s • esc to interrupt)", Style::DarkGray),
      ]),
      Line::blank(),
    ]));

    transcript.push_agent_delta("baz");
    transcript.finish_agent_activity();

    assert!(transcript.render(80).ends_with(&[
      Line::blank(),
      Line::from([Span::styled(" bar", Style::DarkGray)]),
      Line::blank(),
      Line::raw(" baz"),
      Line::blank(),
    ]));

    assert_eq!(
      transcript.messages(),
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::Agent(vec![
          AgentMessageContent::Reasoning("bar".into()),
          AgentMessageContent::Text("baz".into()),
        ])
      ]
    );
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

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![AgentMessageContent::ToolCall(
        invocation.clone(),
      )])]
    );

    let result = ToolResult::command(Some(0), "bar\n", "");

    transcript.push_tool_result("foo", result.clone());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![AgentMessageContent::ToolCall(invocation)]),
        result.message("foo")
      ]
    );
  }

  #[test]
  fn reasoning_is_preserved_with_tool_messages() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }),
    };

    transcript.push_agent_reasoning_delta("baz");
    transcript.push_tool_call(invocation.clone());

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![
        AgentMessageContent::Reasoning("baz".into()),
        AgentMessageContent::ToolCall(invocation),
      ])]
    );
  }

  #[test]
  fn adjacent_tool_messages_share_an_agent_message() {
    let mut transcript = Transcript::default();

    let foo = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }),
    };

    let baz = ToolInvocation {
      id: "baz".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }),
    };

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_tool_call(foo.clone());
    transcript.push_tool_call(baz.clone());

    let foo_result = ToolResult::content("bar");
    let baz_result = ToolResult::content("bar");

    transcript.push_tool_result("foo", foo_result.clone());
    transcript.push_tool_result("baz", baz_result.clone());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![
          AgentMessageContent::Reasoning("foo".into()),
          AgentMessageContent::ToolCall(foo),
          AgentMessageContent::ToolCall(baz),
        ]),
        foo_result.message("foo"),
        baz_result.message("baz"),
      ]
    );
  }

  #[test]
  fn tool_rendering_spacing() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::ListFiles(ListFilesTool {
        cwd: Some(".".into()),
      }),
    };

    transcript.push_agent("foo");
    transcript.push_tool_call(invocation);

    transcript
      .push_tool_result("bar", ToolResult::command(Some(0), "baz\n", ""));

    transcript.finish_agent_activity();

    assert_eq!(
      transcript.render(80),
      [
        Line::blank(),
        Line::raw(" foo"),
        Line::blank(),
        Line::from([
          Span::raw(" "),
          Span::styled("●", Style::GreenBold),
          Span::raw(" "),
          Span::raw("Listed files in ."),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("baz", Style::DarkGray),
        ]),
        Line::blank(),
      ]
    );
  }
}
