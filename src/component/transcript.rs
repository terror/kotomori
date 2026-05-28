use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  pub(crate) fn new(state: &'a Transcript) -> Self {
    Self { state }
  }

  fn render_agent_activity(&self) -> Vec<Line> {
    let mut lines = Vec::new();

    let working = || {
      Line::from([
        Span::styled(
          Transcript::spinner(self.state.active_frame),
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
  fn render_active_waiting_spinner() {
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

    assert!(
      TranscriptComponent::new(&transcript)
        .render(80)
        .ends_with(&[
          Line::from([
            Span::styled("✶", Style::CyanBold),
            Span::styled(" Working...", Style::Gray),
            Span::styled(" (1m 51s • esc to interrupt)", Style::DarkGray),
          ]),
          Line::blank(),
        ])
    );

    transcript.push_agent_delta("bar");

    assert!(
      TranscriptComponent::new(&transcript)
        .render(80)
        .ends_with(&[Line::raw(" bar"), Line::blank(),])
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
  fn render_interrupted_entry() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.interrupt();

    assert!(TranscriptComponent::new(&transcript).render(80).ends_with(&[
      Line::blank(),
      Line::from([Span::styled(
        "■ Conversation interrupted, tell the model what to do differently.",
        Style::RedBold,
      )]),
      Line::blank(),
    ]));
  }

  #[test]
  fn render_reasoning_activity() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.tick(Duration::from_millis(120));
    transcript.push_agent_reasoning_delta("bar");

    assert!(
      TranscriptComponent::new(&transcript)
        .render(80)
        .ends_with(&[
          Line::from([Span::styled(" bar", Style::DarkGray)]),
          Line::blank(),
          Line::from([
            Span::styled("✧", Style::CyanBold),
            Span::styled(" Working...", Style::Gray),
            Span::styled(" (0s • esc to interrupt)", Style::DarkGray),
          ]),
          Line::blank(),
        ])
    );

    transcript.push_agent_delta("baz");
    transcript.finish_agent_activity();

    assert!(
      TranscriptComponent::new(&transcript)
        .render(80)
        .ends_with(&[
          Line::blank(),
          Line::from([Span::styled(" bar", Style::DarkGray)]),
          Line::blank(),
          Line::raw(" baz"),
          Line::blank(),
        ])
    );
  }

  #[test]
  fn render_tool_spacing() {
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
  fn render_user_entry_uses_width() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::User("foobar".into())]);

    assert_eq!(
      TranscriptComponent::new(&transcript).render(3),
      [
        Line::from([Span::styled("───", Style::DarkGray)]),
        Line::raw("foo"),
        Line::raw("bar"),
        Line::from([Span::styled("───", Style::DarkGray)]),
      ]
    );
  }
}
