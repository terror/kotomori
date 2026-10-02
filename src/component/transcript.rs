use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  run: Option<&'a Run>,
  state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  const FRAMES: &'static [&'static str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  fn agent_activity(&self) -> StackComponent<'a> {
    let Some(run) = self.run else {
      return StackComponent::default();
    };

    let stack = StackComponent::default().push(Self::draft(&run.message, &[]));

    match run.activity {
      AgentActivity::Reasoning | AgentActivity::Waiting => {
        stack.push(LineComponent::from([
          Span::styled(
            Self::FRAMES[run.frame % Self::FRAMES.len()],
            Style::Accent,
          ),
          Span::styled(" Working...", Style::Secondary),
          Span::styled(
            format!(" ({} • Esc to interrupt)", run.elapsed.format()),
            Style::Muted,
          ),
        ]))
      }
      AgentActivity::Streaming => stack,
    }
  }

  fn agent_content(
    content: &[AssistantContent],
    following: &'a [TranscriptEntry],
  ) -> StackComponent<'a> {
    content
      .iter()
      .fold(StackComponent::default(), |stack, content| match content {
        AssistantContent::Reasoning(reasoning) => {
          stack.push_spaced(LinesComponent::raw(reasoning.text().lines()))
        }
        AssistantContent::Text(text) => {
          stack.push_spaced(MarkdownComponent::new(&text.text))
        }
        AssistantContent::ToolCall(call) => {
          let result = following
            .iter()
            .take_while(|entry| !matches!(entry, TranscriptEntry::Draft(_)))
            .filter_map(TranscriptEntry::message)
            .take_while(|message| matches!(message, Message::User(_)))
            .find_map(|message| match message {
              Message::User(content) => {
                content.iter().find_map(|content| match content {
                  UserMessageContent::ToolResult {
                    call: id, result, ..
                  } if *id == call.id => Some(result),
                  _ => None,
                })
              }
              Message::Agent(_) => None,
            });

          match ToolInvocationKind::decode(call.clone()) {
            Ok(invocation) => stack.push_spaced(
              TranscriptToolInvocationComponent::new(invocation, result),
            ),
            Err(_) => stack.push_spaced(LineComponent::raw(format!(
              "{} {}",
              call.function.name, call.function.arguments
            ))),
          }
        }
        AssistantContent::Image(_) => stack,
      })
  }

  fn draft(
    buffer: &MessageBuffer,
    following: &'a [TranscriptEntry],
  ) -> StackComponent<'a> {
    buffer
      .preview()
      .fold(StackComponent::default(), |stack, block| match block {
        MessageDraft::Content(content) => stack.push_spaced(
          Self::agent_content(slice::from_ref(content), following),
        ),
        MessageDraft::Reasoning(reasoning) => {
          stack.push_spaced(LinesComponent::raw(reasoning.lines()))
        }
      })
  }

  fn entries(&self) -> StackComponent<'a> {
    self.state.entries.iter().enumerate().fold(
      StackComponent::default(),
      |stack, (index, entry)| match entry {
        TranscriptEntry::Draft(buffer) => {
          stack.push_spaced(Self::draft(
            buffer,
            &self.state.entries[index + 1..],
          ))
        }
        TranscriptEntry::Error(error) => {
          stack.push_spaced(TranscriptErrorComponent::new(error))
        }
        TranscriptEntry::Interrupted => {
          stack.push_spaced(LineComponent::from([Span::styled(
            "■ Conversation interrupted, tell the model what to do differently.",
            Style::Danger,
          )]))
        }
        TranscriptEntry::Message(Message::Agent(message)) => {
          stack.push_spaced(Self::agent_content(
            &message.content,
            &self.state.entries[index + 1..],
          ))
        }
        TranscriptEntry::Message(Message::User(content)) => {
          content.iter().filter_map(UserMessageContent::text).fold(
            stack,
            |stack, text| stack.push(GutterComponent::new(
              LinesComponent::raw(text.split('\n')),
              Span::styled("│ ", Style::Accent),
            )),
          )
        }
        TranscriptEntry::Notice(notice) => {
          stack.push_spaced(LinesComponent::raw(notice.lines()))
        }
      },
    )
  }

  pub(crate) fn new(state: &'a Transcript, run: Option<&'a Run>) -> Self {
    Self { run, state }
  }
}

impl Component for TranscriptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self
      .entries()
      .push_spaced(self.agent_activity())
      .render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
  fn render_active_content_in_order() {
    let mut run = Run::new(0);

    run.update_many(&[
      MessageUpdate::ReasoningDelta {
        index: 0,
        delta: "foo".into(),
      },
      MessageUpdate::Text {
        delta: "bar".into(),
        index: 1,
      },
      MessageUpdate::ReasoningDelta {
        index: 2,
        delta: "baz".into(),
      },
      MessageUpdate::Text {
        delta: "qux".into(),
        index: 3,
      },
    ]);

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
  fn render_active_reasoning() {
    let mut run = Run {
      elapsed: Duration::from_secs(61),
      frame: 1,
      ..Run::new(0)
    };

    run.update(MessageUpdate::ReasoningDelta {
      index: 0,
      delta: "foo\nbar".into(),
    });

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
    let mut run = Run::new(0);

    run.update(MessageUpdate::Text {
      delta: "foo\nbar".into(),
      index: 0,
    });

    assert_eq!(
      TranscriptComponent::new(&Transcript::default(), Some(&run)).render(80),
      [LineComponent::raw("foo bar"), LineComponent::blank()]
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
  fn render_adjacent_non_user_entries_have_single_blank_line() {
    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
        "foo",
      )])),
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
  fn render_agent_entry_handles_multiline_content() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::agent(vec![AssistantContent::text("foo\nbar")]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(2),
      [
        LineComponent::raw("fo"),
        LineComponent::raw("o "),
        LineComponent::raw("ba"),
        LineComponent::raw("r"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_draft_content_in_order() {
    let mut buffer = MessageBuffer::default();

    buffer.apply_many(&[
      MessageUpdate::Text {
        delta: "foo".into(),
        index: 0,
      },
      MessageUpdate::ReasoningDelta {
        delta: "bar".into(),
        index: 1,
      },
      MessageUpdate::Text {
        delta: "baz".into(),
        index: 2,
      },
    ]);

    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Draft(buffer)]);

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
  fn render_empty_transcript() {
    let transcript = Transcript::default();

    assert_eq!(TranscriptComponent::new(&transcript, None).render(80), []);
  }

  #[test]
  fn render_entry_spacing() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::agent(vec![
        AssistantContent::text("foo"),
        AssistantContent::reasoning("foo", "bar"),
        AssistantContent::text("baz"),
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
  fn render_markdown_in_active_draft_and_completed_messages() {
    #[track_caller]
    fn case(text: &str, span: Span) {
      let mut run = Run::new(0);

      run.update(MessageUpdate::Text {
        delta: text.into(),
        index: 0,
      });

      let expected = [LineComponent::from([span]), LineComponent::blank()];

      assert_eq!(
        TranscriptComponent::new(&Transcript::default(), Some(&run)).render(80),
        expected,
      );

      for entry in [
        TranscriptEntry::Draft(run.message),
        TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
          text,
        )])),
      ] {
        assert_eq!(
          TranscriptComponent::new(
            &Transcript::with_entries(vec![entry]),
            None
          )
          .render(80),
          expected,
        );
      }
    }

    case("**foo", Span::raw("**foo"));

    case(
      "**foo**",
      Span::styled("foo", Style::Markdown(anstyle::Style::new().bold())),
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
    let mut reasoning =
      Reasoning::new_with_signature("foo\nbar", Some("baz".into()));

    reasoning.content.extend([
      ReasoningContent::Summary("qux".into()),
      ReasoningContent::Encrypted("quux".into()),
      ReasoningContent::Redacted {
        data: "quuz".into(),
      },
    ]);

    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Message(Message::agent(
        vec![AssistantContent::Reasoning(reasoning.sealed("foo"))],
      ))]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::raw("qux"),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_after_agent() {
    let invocation = ToolInvocation::new(
      "bar",
      ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    );

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::agent(vec![
        AssistantContent::text("foo"),
        AssistantContent::ToolCall(invocation.protocol.clone()),
        AssistantContent::text("bar"),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: invocation.protocol.id.clone(),
          name: invocation.protocol.function.name.clone(),
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
    let invocation = ToolInvocation::new(
      "bar",
      ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    );

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "foo".into(),
      )])),
      TranscriptEntry::Message(Message::agent(vec![
        AssistantContent::ToolCall(invocation.protocol.clone()),
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
    let invocation = ToolInvocation::new(
      "bar",
      ToolInvocationKind::Command(CommandTool {
        command: "foo bar".into(),
        cwd: Some("baz".into()),
      }),
    );

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::agent(vec![
        AssistantContent::ToolCall(invocation.protocol.clone()),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: invocation.protocol.id.clone(),
          name: invocation.protocol.function.name.clone(),
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
    let invocation = ToolInvocation::new(
      "bar",
      ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    );

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::agent(vec![
        AssistantContent::ToolCall(invocation.protocol.clone()),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: invocation.protocol.id.clone(),
          name: invocation.protocol.function.name.clone(),
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
          Span::raw(" Ran rg -"),
        ]),
        LineComponent::raw("-files"),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("foo..."),
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
          Span::styled("... 1 ", Style::Muted),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::styled("more l", Style::Muted),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::styled("ine", Style::Muted),
        ]),
        LineComponent::blank(),
      ]
    );
  }

  #[test]
  fn render_tool_entry_pending() {
    let invocation = ToolInvocation::new(
      "bar",
      ToolInvocationKind::Command(CommandTool {
        command: "rg --files".into(),
        cwd: None,
      }),
    );

    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Message(Message::agent(
        vec![AssistantContent::ToolCall(invocation.protocol.clone())],
      ))]);

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
    let invocation = ToolInvocation::new(
      "foo",
      ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    );

    let message =
      Message::agent(vec![AssistantContent::ToolCall(invocation.protocol)]);

    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(message.clone()),
      TranscriptEntry::Message(message),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("baz"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult::default(),
        },
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
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
  fn render_unknown_tool() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::agent(vec![AssistantContent::ToolCall(
        ::rig::message::ToolCall::from_wire(
          "foo",
          ToolFunction {
            arguments: serde_json::json!({"bar": "baz"}),
            name: ToolName::new("foo").unwrap(),
          },
        ),
      )]),
    )]);

    assert_eq!(
      TranscriptComponent::new(&transcript, None).render(80),
      [
        LineComponent::raw("foo {\"bar\":\"baz\"}"),
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
