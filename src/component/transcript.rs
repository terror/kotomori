use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  run: Option<&'a Run>,
  state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  const FRAMES: &'static [&'static str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  fn ensure_trailing_blank_line(lines: &mut Vec<LineComponent>) {
    if lines.last().is_some_and(|line| !line.is_blank()) {
      lines.push(LineComponent::blank());
    }
  }

  pub(crate) fn new(state: &'a Transcript, run: Option<&'a Run>) -> Self {
    Self { run, state }
  }

  fn render_agent_activity(&self, width: u16) -> Vec<LineComponent> {
    let Some(run) = self.run else {
      return Vec::new();
    };

    let mut lines = Self::render_agent_content(&run.content, &[], width);

    let working = || {
      LineComponent::from([
        Span::styled(
          Self::FRAMES[run.frame % Self::FRAMES.len()],
          Style::Accent,
        ),
        Span::styled(" Working...", Style::Secondary),
        Span::styled(
          format!(" ({} • Esc to interrupt)", run.elapsed.format()),
          Style::Muted,
        ),
      ])
    };

    match &run.activity {
      AgentActivity::Reasoning(reasoning) => {
        lines.extend(reasoning.lines().map(LineComponent::raw));

        lines.extend([LineComponent::blank(), working()]);
      }
      AgentActivity::Streaming(message) => {
        lines.extend(message.lines().map(LineComponent::raw));
      }
      AgentActivity::Waiting => {
        lines.push(working());
      }
    }

    lines
  }

  fn render_agent_content(
    content: &[AgentMessageContent],
    following: &[TranscriptEntry],
    width: u16,
  ) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    for content in content {
      Self::ensure_trailing_blank_line(&mut lines);

      match content {
        AgentMessageContent::Reasoning(text)
        | AgentMessageContent::Text(text) => {
          lines.extend(text.lines().map(LineComponent::raw));
        }
        AgentMessageContent::ToolCall(invocation) => {
          let result = following
            .iter()
            .filter_map(TranscriptEntry::message)
            .take_while(|message| matches!(message, Message::User(_)))
            .find_map(|message| match message {
              Message::User(content) => {
                content.iter().find_map(|content| match content {
                  UserMessageContent::ToolResult { id, result }
                    if *id == invocation.id =>
                  {
                    Some(result)
                  }
                  _ => None,
                })
              }
              Message::Agent(_) => None,
            });

          lines.extend(
            TranscriptToolInvocationComponent::new(invocation, result)
              .render(width),
          );
        }
      }

      Self::ensure_trailing_blank_line(&mut lines);
    }

    lines
  }

  fn render_entries(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    for (index, entry) in self.state.entries.iter().enumerate() {
      match entry {
        TranscriptEntry::Error(error) => {
          Self::ensure_trailing_blank_line(&mut lines);
          lines.extend(TranscriptErrorComponent::new(error).render(width));
          Self::ensure_trailing_blank_line(&mut lines);
        }
        TranscriptEntry::Interrupted => {
          Self::ensure_trailing_blank_line(&mut lines);

          lines.push(LineComponent::from([Span::styled(
            "■ Conversation interrupted, tell the model what to do differently.",
            Style::Danger,
          )]));

          Self::ensure_trailing_blank_line(&mut lines);
        }
        TranscriptEntry::Message(Message::Agent(content)) => {
          Self::ensure_trailing_blank_line(&mut lines);

          lines.extend(Self::render_agent_content(
            content,
            &self.state.entries[index + 1..],
            width,
          ));
        }
        TranscriptEntry::Message(Message::User(content)) => {
          for text in content.iter().filter_map(UserMessageContent::text) {
            lines.extend(
              GutteredLinesComponent::raw(text.split('\n')).render(width),
            );
          }
        }
        TranscriptEntry::Notice(notice) => {
          Self::ensure_trailing_blank_line(&mut lines);
          lines.extend(notice.lines().map(LineComponent::raw));
          Self::ensure_trailing_blank_line(&mut lines);
        }
      }
    }

    lines
  }
}

impl Component for TranscriptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = self.render_entries(width);

    let activity = self.render_agent_activity(width);

    if !activity.is_empty() {
      if !lines.is_empty() {
        Self::ensure_trailing_blank_line(&mut lines);
      }

      lines.extend(activity);

      Self::ensure_trailing_blank_line(&mut lines);
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn render_active_reasoning() {
    let run = Run {
      activity: AgentActivity::Reasoning("foo\nbar".into()),
      elapsed: Duration::from_secs(61),
      frame: 1,
      ..Run::new(0)
    };

    assert_eq!(
      TranscriptComponent::new(&Transcript::default(), Some(&run)).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("✧", Style::Accent),
          Span::styled(" Working...", Style::Secondary),
          Span::styled(" (1m 1s • Esc to interrupt)", Style::Muted),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_active_streaming() {
    let run = Run {
      activity: AgentActivity::Streaming("foo\nbar".into()),
      ..Run::new(0)
    };

    assert_eq!(
      TranscriptComponent::new(&Transcript::default(), Some(&run)).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank()
      ]
    );
  }

  #[test]
  fn render_active_content_in_order() {
    let mut run = Run::new(0);

    run.push_reasoning_delta("foo");
    run.push_delta("bar");
    run.push_reasoning_delta("baz");
    run.push_delta("qux");

    assert_eq!(
      TranscriptComponent::new(&Transcript::default(), Some(&run)).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::raw("bar"),
        LineComponent::blank(),
        LineComponent::raw("baz"),
        LineComponent::blank(),
        LineComponent::raw("qux"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_active_waiting() {
    let run = Run {
      elapsed: Duration::from_secs(111),
      frame: 2,
      ..Run::new(0)
    };

    assert_eq!(
      TranscriptComponent::new(&Transcript::default(), Some(&run)).render(80),
      [
        LineComponent::from([
          Span::styled("✶", Style::Accent),
          Span::styled(" Working...", Style::Secondary),
          Span::styled(" (1m 51s • Esc to interrupt)", Style::Muted),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_active_activity_is_separated_from_user_entry() {
    let run = Run::new(0);

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::User(vec![UserMessageContent::Text("foo".into())]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, Some(&run)).render(80),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("✦", Style::Accent),
          Span::styled(" Working...", Style::Secondary),
          Span::styled(" (0s • Esc to interrupt)", Style::Muted),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_agent_entry_handles_multiline_content() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::Agent(vec![AgentMessageContent::Text("foo\nbar".into())]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_empty_transcript() {
    let transcript = Transcript::default();

    assert_eq!(TranscriptComponent::new(&transcript, None).render(80), []);
  }

  #[test]
  fn render_entry_spacing() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::Agent(vec![
        AgentMessageContent::Text("foo".into()),
        AgentMessageContent::Reasoning("bar".into()),
        AgentMessageContent::Text("baz".into()),
      ]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::raw("bar"),
        LineComponent::blank(),
        LineComponent::raw("baz"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_adjacent_non_user_entries_have_single_blank_line() {
    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::Agent(vec![
        AgentMessageContent::Text("foo".into()),
      ])),
      TranscriptEntry::Interrupted,
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::from([Span::styled(
          "■ Conversation interrupted, tell the model what to do differently.",
          Style::Danger,
        )]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_error_entry() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Error("foo".into())]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::from([
          Span::styled("●", Style::Danger),
          Span::raw(" "),
          Span::raw("Error"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("foo"),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_interrupted_entry() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Interrupted]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::from([Span::styled(
          "■ Conversation interrupted, tell the model what to do differently.",
          Style::Danger,
        )]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_notice_entry_handles_multiline_content() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Notice(
      "foo\nbar".into(),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_reasoning_entry_handles_multiline_content() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::Agent(vec![AgentMessageContent::Reasoning("foo\nbar".into())]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_after_agent() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    };

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::Agent(vec![
        AgentMessageContent::Text("foo".into()),
        AgentMessageContent::ToolCall(invocation.clone()),
        AgentMessageContent::Text("bar".into()),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: invocation.id.clone(),
          result: ToolResult {
            exit_status: Some(0),
            outcome: ToolOutcome::Success,
            stdout: Some("baz\n".into()),
            ..Default::default()
          },
        },
      ])),
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("●", Style::Success),
          Span::raw(" "),
          Span::raw("Ran rg --files"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("baz"),
        ]),
        LineComponent::blank(),
        LineComponent::raw("bar"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_after_user() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    };

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "foo".into(),
      )])),
      TranscriptEntry::Message(Message::Agent(vec![
        AgentMessageContent::ToolCall(invocation.clone()),
      ])),
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("●", Style::Accent),
          Span::raw(" "),
          Span::raw("Running rg --files"),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_failed() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "foo bar".into(),
        cwd: Some("baz".into()),
      }),
    };

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::Agent(vec![
        AgentMessageContent::ToolCall(invocation.clone()),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: invocation.id.clone(),
          result: ToolResult {
            exit_status: Some(1),
            stderr: Some("quux".into()),
            stdout: Some("qux\n".into()),
            ..Default::default()
          },
        },
      ])),
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::from([
          Span::styled("●", Style::Danger),
          Span::raw(" "),
          Span::raw("Failed running foo bar"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::styled("cwd ", Style::Muted),
          Span::raw("baz"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::styled("exit ", Style::Muted),
          Span::raw("1"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("qux"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("quux"),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_limits_output() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    };

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::Agent(vec![
        AgentMessageContent::ToolCall(invocation.clone()),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: invocation.id.clone(),
          result: ToolResult {
            exit_status: Some(0),
            outcome: ToolOutcome::Success,
            stdout: Some("foobarbaz\n\nbar\nbaz\nqux\n".into()),
            ..Default::default()
          },
        },
      ])),
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(10),
      [
        LineComponent::from([
          Span::styled("●", Style::Success),
          Span::raw(" "),
          Span::raw("Ran rg --files"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("fooba..."),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("bar"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("baz"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::styled("... 1 more line", Style::Muted),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_pending() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    };

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::Agent(vec![AgentMessageContent::ToolCall(invocation.clone())]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::from([
          Span::styled("●", Style::Accent),
          Span::raw(" "),
          Span::raw("Running rg --files"),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_results_are_scoped_to_the_agent_message() {
    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    let message =
      Message::Agent(vec![AgentMessageContent::ToolCall(invocation)]);

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(message.clone()),
      TranscriptEntry::Message(message),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: "baz".into(),
          result: ToolResult::default(),
        },
        UserMessageContent::ToolResult {
          id: "foo".into(),
          result: ToolResult {
            outcome: ToolOutcome::Success,
            ..Default::default()
          },
        },
      ])),
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::from([
          Span::styled("●", Style::Accent),
          Span::raw(" "),
          Span::raw("Running bar"),
        ]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("●", Style::Success),
          Span::raw(" "),
          Span::raw("Ran bar"),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_user_entry_uses_width() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::User(vec![UserMessageContent::Text("foobar\nbaz".into())]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(5),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("bar"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("baz"),
        ]),
      ]
    );
  }
}
