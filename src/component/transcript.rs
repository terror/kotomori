use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  const FRAMES: &'static [&'static str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  fn ensure_trailing_blank_line(lines: &mut Vec<LineComponent>) {
    if lines.last().is_some_and(|line| !line.is_blank()) {
      lines.push(LineComponent::blank());
    }
  }

  pub(crate) fn new(state: &'a Transcript) -> Self {
    Self { state }
  }

  fn render_agent_activity(&self) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    let working = || {
      LineComponent::from([
        Span::styled(
          Self::FRAMES[self.state.active_frame % Self::FRAMES.len()],
          Style::Accent,
        ),
        Span::styled(" Working...", Style::Secondary),
        Span::styled(
          format!(
            " ({} • Esc to interrupt)",
            self.state.active_elapsed.format()
          ),
          Style::Muted,
        ),
      ])
    };

    match &self.state.active_agent_activity {
      AgentActivity::Idle => {}
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

  fn render_entries(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    for entry in &self.state.entries {
      let separated = !matches!(entry, TranscriptEntry::User(_));

      if separated {
        Self::ensure_trailing_blank_line(&mut lines);
      }

      match entry {
        TranscriptEntry::Agent(content) => {
          lines.extend(content.lines().map(LineComponent::raw));
        }
        TranscriptEntry::Compaction(_) => {
          lines.push(LineComponent::from([Span::styled(
            "◆ Conversation context compacted.",
            Style::Secondary,
          )]));
        }
        TranscriptEntry::Error(error) => {
          lines.extend(TranscriptErrorComponent::new(error).render(width));
        }
        TranscriptEntry::Interrupted => {
          lines.push(LineComponent::from([Span::styled(
            "■ Conversation interrupted, tell the model what to do differently.",
            Style::Danger,
          )]));
        }
        TranscriptEntry::Notice(notice) => {
          lines.extend(notice.lines().map(LineComponent::raw));
        }
        TranscriptEntry::Reasoning(reasoning) => {
          lines.extend(reasoning.lines().map(LineComponent::raw));
        }
        TranscriptEntry::Tool { invocation, result } => {
          lines.extend(
            TranscriptToolInvocationComponent::new(invocation, result.as_ref())
              .render(width),
          );
        }
        TranscriptEntry::User(content) => {
          lines.extend(
            GutteredLinesComponent::raw(content.split('\n')).render(width),
          );
        }
      }

      if separated {
        Self::ensure_trailing_blank_line(&mut lines);
      }
    }

    lines
  }
}

impl Component for TranscriptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = self.render_entries(width);

    let activity = self.render_agent_activity();

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
    let transcript = Transcript {
      active_agent_activity: AgentActivity::Reasoning("foo\nbar".into()),
      active_elapsed: Duration::from_secs(61),
      active_frame: 1,
      entries: Vec::new(),
    };

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
    let transcript = Transcript {
      active_agent_activity: AgentActivity::Streaming("foo\nbar".into()),
      active_elapsed: Duration::ZERO,
      active_frame: 0,
      entries: Vec::new(),
    };

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank()
      ]
    );
  }

  #[test]
  fn render_active_waiting() {
    let transcript = Transcript {
      active_agent_activity: AgentActivity::Waiting,
      active_elapsed: Duration::from_secs(111),
      active_frame: 2,
      entries: Vec::new(),
    };

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
    let transcript = Transcript {
      active_agent_activity: AgentActivity::Waiting,
      active_elapsed: Duration::ZERO,
      active_frame: 0,
      entries: vec![TranscriptEntry::User("hello".into())],
    };

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("hello"),
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
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Agent("foo\nbar".into())]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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

    assert_eq!(TranscriptComponent::new(&transcript).render(80), []);
  }

  #[test]
  fn render_entry_spacing() {
    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Agent("foo".into()),
      TranscriptEntry::Reasoning("bar".into()),
      TranscriptEntry::Agent("baz".into()),
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
      TranscriptEntry::Agent("foo".into()),
      TranscriptEntry::Interrupted,
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
      TranscriptComponent::new(&transcript).render(80),
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
      TranscriptComponent::new(&transcript).render(80),
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
      TranscriptComponent::new(&transcript).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_reasoning_entry_handles_multiline_content() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Reasoning(
        "foo\nbar".into(),
      )]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
      TranscriptEntry::Agent("foo".into()),
      TranscriptEntry::Tool {
        invocation,
        result: Some(ToolResult {
          exit_status: Some(0),
          outcome: ToolOutcome::Success,
          stdout: Some("baz\n".into()),
          ..Default::default()
        }),
      },
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
      TranscriptEntry::User("foo".into()),
      TranscriptEntry::Tool {
        invocation,
        result: None,
      },
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: Some(ToolResult {
        exit_status: Some(1),
        stderr: Some("quux".into()),
        stdout: Some("qux\n".into()),
        ..Default::default()
      }),
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: Some(ToolResult {
        exit_status: Some(0),
        outcome: ToolOutcome::Success,
        stdout: Some("foobarbaz\n\nbar\nbaz\nqux\n".into()),
        ..Default::default()
      }),
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(10),
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

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: None,
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
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
  fn render_user_entry_uses_width() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::User(
      "foobar\nbaz".into(),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(5),
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
