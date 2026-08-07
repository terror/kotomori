use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  const FRAMES: &'static [&'static str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  fn ensure_trailing_blank_line(lines: &mut Vec<LineComponent>) {
    if !lines.last().is_some_and(LineComponent::is_blank) {
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
          Style::CyanBold,
        ),
        Span::styled(" Working...", Style::Gray),
        Span::styled(
          format!(
            " ({} • esc to interrupt)",
            self.state.active_elapsed.format()
          ),
          Style::DarkGray,
        ),
      ])
    };

    match &self.state.active_agent_activity {
      AgentActivity::Idle => {}
      AgentActivity::Reasoning(reasoning) => {
        lines.extend(reasoning.lines().map(|line| {
          LineComponent::from([Span::styled(
            format!(" {line}"),
            Style::DarkGray,
          )])
        }));

        lines.extend([
          LineComponent::blank(),
          working(),
          LineComponent::blank(),
        ]);
      }
      AgentActivity::Streaming(message) => {
        lines.extend(
          message
            .lines()
            .map(|line| LineComponent::raw(format!(" {line}"))),
        );

        lines.push(LineComponent::blank());
      }
      AgentActivity::Waiting => {
        lines.extend([working(), LineComponent::blank()]);
      }
    }

    lines
  }

  fn render_entries(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    for entry in &self.state.entries {
      match entry {
        TranscriptEntry::Agent(content) => {
          if !matches!(lines.last(), Some(line) if line == &LineComponent::blank())
          {
            lines.push(LineComponent::blank());
          }

          lines.extend(
            content
              .lines()
              .map(|line| LineComponent::raw(format!(" {line}"))),
          );

          lines.push(LineComponent::blank());
        }
        TranscriptEntry::Error(error) => {
          if !matches!(lines.last(), Some(line) if line == &LineComponent::blank())
          {
            lines.push(LineComponent::blank());
          }

          lines.extend(TranscriptErrorComponent::new(error).render(width));
        }
        TranscriptEntry::Interrupted => {
          lines.extend([
            LineComponent::blank(),
            LineComponent::from([Span::styled(
              "■ Conversation interrupted, tell the model what to do differently.",
              Style::RedBold,
            )]),
            LineComponent::blank(),
          ]);
        }
        TranscriptEntry::Reasoning(reasoning) => {
          Self::ensure_trailing_blank_line(&mut lines);

          lines.extend(reasoning.lines().map(|line| {
            LineComponent::from([Span::styled(
              format!(" {line}"),
              Style::DarkGray,
            )])
          }));

          lines.push(LineComponent::blank());
        }
        TranscriptEntry::Tool { invocation, result } => {
          if !matches!(lines.last(), Some(line) if line == &LineComponent::blank())
          {
            lines.push(LineComponent::blank());
          }

          lines.extend(
            TranscriptToolInvocationComponent::new(invocation, result.as_ref())
              .render(width),
          );
        }
        TranscriptEntry::User(content) => {
          lines.extend(
            FramedLinesComponent::raw(content.split('\n')).render(width),
          );
        }
      }
    }

    lines
  }
}

impl Component for TranscriptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self
      .render_entries(width)
      .into_iter()
      .chain(self.render_agent_activity())
      .collect()
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
        LineComponent::from([Span::styled(" foo", Style::DarkGray)]),
        LineComponent::from([Span::styled(" bar", Style::DarkGray)]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("✧", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(" (1m 1s • esc to interrupt)", Style::DarkGray),
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
        LineComponent::raw(" foo"),
        LineComponent::raw(" bar"),
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
          Span::styled("✶", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(" (1m 51s • esc to interrupt)", Style::DarkGray),
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
        LineComponent::blank(),
        LineComponent::raw(" foo"),
        LineComponent::raw(" bar"),
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
        LineComponent::blank(),
        LineComponent::raw(" foo"),
        LineComponent::blank(),
        LineComponent::from([Span::styled(" bar", Style::DarkGray)]),
        LineComponent::blank(),
        LineComponent::raw(" baz"),
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
        LineComponent::blank(),
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::RedBold),
          Span::raw(" "),
          Span::raw("Error"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
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
        LineComponent::blank(),
        LineComponent::from([Span::styled(
          "■ Conversation interrupted, tell the model what to do differently.",
          Style::RedBold,
        )]),
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
        LineComponent::blank(),
        LineComponent::from([Span::styled(" foo", Style::DarkGray)]),
        LineComponent::from([Span::styled(" bar", Style::DarkGray)]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_after_agent() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["--files".into()],
        cwd: None,
        program: "rg".into(),
      }),
    };

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Agent("foo".into()),
      TranscriptEntry::Tool {
        invocation,
        result: Some(ToolResult::command(Some(0), "baz\n", "")),
      },
    ]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
      [
        LineComponent::blank(),
        LineComponent::raw(" foo"),
        LineComponent::blank(),
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::GreenBold),
          Span::raw(" "),
          Span::raw("Ran rg --files"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("baz", Style::DarkGray),
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
        arguments: vec!["--files".into()],
        cwd: None,
        program: "rg".into(),
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
        LineComponent::from([Span::styled("─".repeat(80), Style::DarkGray)]),
        LineComponent::raw("foo"),
        LineComponent::from([Span::styled("─".repeat(80), Style::DarkGray)]),
        LineComponent::blank(),
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::CyanBold),
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
        arguments: vec!["bar".into()],
        cwd: Some("baz".into()),
        program: "foo".into(),
      }),
    };

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: Some(ToolResult::command(Some(1), "qux\n", "quux")),
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
      [
        LineComponent::blank(),
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::RedBold),
          Span::raw(" "),
          Span::raw("Failed running foo bar"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("cwd ", Style::DarkGray),
          Span::raw("baz"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("exit ", Style::DarkGray),
          Span::raw("1"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("qux", Style::DarkGray),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("quux", Style::DarkGray),
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
        arguments: vec!["--files".into()],
        cwd: None,
        program: "rg".into(),
      }),
    };

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: Some(ToolResult::command(
        Some(0),
        "foobarbaz\n\nbar\nbaz\nqux\n",
        "",
      )),
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(10),
      [
        LineComponent::blank(),
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::GreenBold),
          Span::raw(" "),
          Span::raw("Ran rg --files"),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("fooba...", Style::DarkGray),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("bar", Style::DarkGray),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("baz", Style::DarkGray),
        ]),
        LineComponent::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("... 1 more line", Style::DarkGray),
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
        arguments: vec!["--files".into()],
        cwd: None,
        program: "rg".into(),
      }),
    };

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: None,
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
      [
        LineComponent::blank(),
        LineComponent::from([
          Span::raw(" "),
          Span::styled("●", Style::CyanBold),
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
      TranscriptComponent::new(&transcript).render(3),
      [
        LineComponent::from([Span::styled("───", Style::DarkGray)]),
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::raw("baz"),
        LineComponent::from([Span::styled("───", Style::DarkGray)]),
      ]
    );
  }
}
