use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  const FRAMES: &'static [&'static str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  pub(crate) fn new(state: &'a Transcript) -> Self {
    Self { state }
  }

  fn render_agent_activity(&self) -> Vec<Line> {
    let mut lines = Vec::new();

    let working = || {
      Line::from([
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
          Line::from([Span::styled(format!(" {line}"), Style::DarkGray)])
        }));

        lines.extend([Line::blank(), working(), Line::blank()]);
      }
      AgentActivity::Streaming(message) => {
        lines.extend(message.lines().map(|line| Line::raw(format!(" {line}"))));

        lines.push(Line::blank());
      }
      AgentActivity::Waiting => {
        lines.extend([working(), Line::blank()]);
      }
    }

    lines
  }

  fn render_entries(&self, width: u16) -> Vec<Line> {
    let mut lines = Vec::new();

    for entry in &self.state.entries {
      match entry {
        TranscriptEntry::Agent(content) => {
          if !matches!(lines.last(), Some(line) if line == &Line::blank()) {
            lines.push(Line::blank());
          }

          lines
            .extend(content.lines().map(|line| Line::raw(format!(" {line}"))));

          lines.push(Line::blank());
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
          if !matches!(lines.last(), Some(line) if line == &Line::blank()) {
            lines.push(Line::blank());
          }

          lines.extend(reasoning.lines().map(|line| {
            Line::from([Span::styled(format!(" {line}"), Style::DarkGray)])
          }));

          lines.push(Line::blank());
        }
        TranscriptEntry::Tool { invocation, result } => {
          if !matches!(lines.last(), Some(line) if line == &Line::blank()) {
            lines.push(Line::blank());
          }

          lines.extend(
            TranscriptToolInvocationComponent::new(invocation, result.as_ref())
              .render(width),
          );
        }
        TranscriptEntry::User(content) => {
          lines.extend(
            MessageComponent::new(&Message::User(vec![
              UserMessageContent::Text(content.clone()),
            ]))
            .render(width),
          );
        }
      }
    }

    lines
  }
}

impl Component for TranscriptComponent<'_> {
  fn render(&self, width: u16) -> Vec<Line> {
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
        Line::from([Span::styled(" foo", Style::DarkGray)]),
        Line::from([Span::styled(" bar", Style::DarkGray)]),
        Line::blank(),
        Line::from([
          Span::styled("✧", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(" (1m 1s • esc to interrupt)", Style::DarkGray),
        ]),
        Line::blank(),
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
      [Line::raw(" foo"), Line::raw(" bar"), Line::blank()]
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
        Line::from([
          Span::styled("✶", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(" (1m 51s • esc to interrupt)", Style::DarkGray),
        ]),
        Line::blank(),
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
        Line::blank(),
        Line::raw(" foo"),
        Line::raw(" bar"),
        Line::blank(),
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
        Line::blank(),
        Line::raw(" foo"),
        Line::blank(),
        Line::from([Span::styled(" bar", Style::DarkGray)]),
        Line::blank(),
        Line::raw(" baz"),
        Line::blank(),
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
        Line::blank(),
        Line::from([Span::styled(
          "■ Conversation interrupted, tell the model what to do differently.",
          Style::RedBold,
        )]),
        Line::blank(),
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
        Line::blank(),
        Line::from([Span::styled(" foo", Style::DarkGray)]),
        Line::from([Span::styled(" bar", Style::DarkGray)]),
        Line::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_after_agent() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::ListFiles(ListFilesTool {
        cwd: Some(".".into()),
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

  #[test]
  fn render_tool_entry_after_user() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::ListFiles(ListFilesTool { cwd: None }),
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
        Line::from([Span::styled("─".repeat(80), Style::DarkGray)]),
        Line::raw("foo"),
        Line::from([Span::styled("─".repeat(80), Style::DarkGray)]),
        Line::blank(),
        Line::from([
          Span::raw(" "),
          Span::styled("●", Style::CyanBold),
          Span::raw(" "),
          Span::raw("Listing files"),
        ]),
        Line::blank(),
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
        Line::blank(),
        Line::from([
          Span::raw(" "),
          Span::styled("●", Style::RedBold),
          Span::raw(" "),
          Span::raw("Failed running foo bar"),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("cwd ", Style::DarkGray),
          Span::raw("baz"),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("exit ", Style::DarkGray),
          Span::raw("1"),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("qux", Style::DarkGray),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("quux", Style::DarkGray),
        ]),
        Line::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_limits_output() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::ListFiles(ListFilesTool { cwd: None }),
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
        Line::blank(),
        Line::from([
          Span::raw(" "),
          Span::styled("●", Style::GreenBold),
          Span::raw(" "),
          Span::raw("Listed files"),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("fooba...", Style::DarkGray),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("bar", Style::DarkGray),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("baz", Style::DarkGray),
        ]),
        Line::from([
          Span::styled("   │ ", Style::DarkGray),
          Span::styled("... 1 more line", Style::DarkGray),
        ]),
        Line::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_pending() {
    let invocation = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::ListFiles(ListFilesTool {
        cwd: Some("baz".into()),
      }),
    };

    let transcript = Transcript::with_entries(vec![TranscriptEntry::Tool {
      invocation,
      result: None,
    }]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(80),
      [
        Line::blank(),
        Line::from([
          Span::raw(" "),
          Span::styled("●", Style::CyanBold),
          Span::raw(" "),
          Span::raw("Listing files in baz"),
        ]),
        Line::blank(),
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
        Line::from([Span::styled("───", Style::DarkGray)]),
        Line::raw("foo"),
        Line::raw("bar"),
        Line::raw("baz"),
        Line::from([Span::styled("───", Style::DarkGray)]),
      ]
    );
  }
}
