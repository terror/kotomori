use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptComponent<'a> {
  pub(crate) reasoning_expanded: bool,
  pub(crate) run: Option<&'a Run>,
  pub(crate) state: &'a Transcript,
}

impl<'a> TranscriptComponent<'a> {
  const FRAMES: &'static [&'static str] = &["✦", "✧", "✶", "✹", "✶", "✧"];

  fn agent_activity(&self) -> StackComponent<'a> {
    let Some(run) = self.run else {
      return StackComponent::default();
    };

    let stack = StackComponent::default()
      .gap(1)
      .push(self.draft(&run.message, &[]));

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
    &self,
    content: &[AssistantContent],
    following: &'a [TranscriptEntry],
  ) -> StackComponent<'a> {
    content
      .iter()
      .fold(
        StackComponent::default().gap(1),
        |stack, content| match content {
          AssistantContent::Reasoning(reasoning) => {
            stack.push(self.reasoning(&reasoning.text()))
          }
          AssistantContent::Text(text) => stack.push(MarkdownComponent {
            text: text.text.clone(),
          }),
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
              Ok(invocation) => stack
                .push(TranscriptToolInvocationComponent { invocation, result }),
              Err(_) => stack.push(LineComponent::raw(format!(
                "{} {}",
                call.function.name, call.function.arguments
              ))),
            }
          }
          AssistantContent::Image(_) => stack,
        },
      )
  }

  fn draft(
    &self,
    buffer: &MessageBuffer,
    following: &'a [TranscriptEntry],
  ) -> StackComponent<'a> {
    buffer
      .preview()
      .fold(
        StackComponent::default().gap(1),
        |stack, block| match block {
          MessageDraft::Content(content) => {
            stack.push(self.agent_content(slice::from_ref(content), following))
          }
          MessageDraft::Reasoning(reasoning) => {
            stack.push(self.reasoning(reasoning))
          }
        },
      )
  }

  fn entries(&self) -> StackComponent<'a> {
    self
      .state
      .entries
      .chunk_by(|left, right| {
        matches!(
          (left, right),
          (
            TranscriptEntry::Message(Message::User(_)),
            TranscriptEntry::Message(Message::User(_))
          )
        )
      })
      .fold(
        (StackComponent::default().gap(1), 0),
        |(stack, index), entries| {
          let group = entries.iter().enumerate().fold(
            StackComponent::default(),
            |stack, (offset, entry)| match entry {
              TranscriptEntry::Draft(buffer) => stack.push(self.draft(
                buffer,
                &self.state.entries[index + offset + 1..],
              )),
              TranscriptEntry::Error(error) => {
                stack.push(TranscriptErrorComponent { error })
              }
              TranscriptEntry::Interrupted => {
                stack.push(LineComponent::from([Span::styled(
                  "■ Conversation interrupted, tell the model what to do differently.",
                  Style::Danger,
                )]))
              }
              TranscriptEntry::Message(Message::Agent(message)) => {
                stack.push(self.agent_content(
                  &message.content,
                  &self.state.entries[index + offset + 1..],
                ))
              }
              TranscriptEntry::Message(Message::User(content)) => {
                content.iter().filter_map(UserMessageContent::text).fold(
                  stack,
                  |stack, text| {
                    stack.push(GutterComponent {
                      component: LinesComponent::raw(text.split('\n')),
                      gutter: Span::styled("│ ", Style::Accent),
                    })
                  },
                )
              }
              TranscriptEntry::Notice(notice) => {
                stack.push(LinesComponent::raw(notice.lines()))
              }
            },
          );

          (stack.push(group), index + entries.len())
        },
      )
      .0
  }

  fn reasoning(&self, reasoning: &str) -> StackComponent<'a> {
    if reasoning.is_empty() {
      return StackComponent::default();
    }

    let stack =
      StackComponent::default().push(LineComponent::from([Span::styled(
        "Thinking...",
        Style::Muted,
      )]));

    if self.reasoning_expanded {
      stack.push(GutterComponent {
        component: LinesComponent::raw(reasoning.lines()),
        gutter: Span::styled("  │ ", Style::Muted),
      })
    } else {
      stack
    }
  }
}

impl Component for TranscriptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self.entries().push(self.agent_activity()).render(width)
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: Some(&run),
        state: &transcript
      }
      .render(80),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: Some(&run),
        state: &Transcript::default()
      }
      .render(80),
      [
        LineComponent::from([Span::styled("Thinking...", Style::Muted,)]),
        LineComponent::blank(),
        LineComponent::raw("bar"),
        LineComponent::blank(),
        LineComponent::from([Span::styled("Thinking...", Style::Muted,)]),
        LineComponent::blank(),
        LineComponent::raw("qux"),
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

    for delta in ["foo\nbar", "\nbaz"] {
      run.update(MessageUpdate::ReasoningDelta {
        index: 0,
        delta: delta.into(),
      });

      assert_eq!(
        TranscriptComponent {
          reasoning_expanded: false,
          run: Some(&run),
          state: &Transcript::default()
        }
        .render(80),
        [
          LineComponent::from([Span::styled("Thinking...", Style::Muted,)]),
          LineComponent::blank(),
          LineComponent::from([
            Span::styled("✧", Style::Accent),
            Span::styled(" Working...", Style::Secondary),
            Span::styled(" (1m 1s • Esc to interrupt)", Style::Muted),
          ]),
        ]
      );
    }
  }

  #[test]
  fn render_active_streaming() {
    let mut run = Run::new(0);

    run.update(MessageUpdate::Text {
      delta: "foo\nbar".into(),
      index: 0,
    });

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: Some(&run),
        state: &Transcript::default()
      }
      .render(80),
      [LineComponent::raw("foo bar")]
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: Some(&run),
        state: &Transcript::default()
      }
      .render(80),
      [LineComponent::from([
        Span::styled("✶", Style::Accent),
        Span::styled(" Working...", Style::Secondary),
        Span::styled(" (1m 51s • Esc to interrupt)", Style::Muted),
      ])]
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::from([Span::styled(
          "■ Conversation interrupted, tell the model what to do differently.",
          Style::Danger,
        )]),
      ]
    );
  }

  #[test]
  fn render_agent_entry_handles_multiline_content() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::agent(vec![AssistantContent::text("foo\nbar")]),
    )]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(2),
      [
        LineComponent::raw("fo"),
        LineComponent::raw("o "),
        LineComponent::raw("ba"),
        LineComponent::raw("r"),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::from([Span::styled("Thinking...", Style::Muted,)]),
        LineComponent::blank(),
        LineComponent::raw("baz"),
      ]
    );
  }

  #[test]
  fn render_empty_reasoning() {
    #[track_caller]
    fn case(reasoning: Reasoning) {
      let transcript =
        Transcript::with_entries(vec![TranscriptEntry::Message(
          Message::agent(vec![AssistantContent::Reasoning(
            reasoning.sealed("foo"),
          )]),
        )]);

      for expanded in [false, true] {
        assert_eq!(
          TranscriptComponent {
            reasoning_expanded: expanded,
            run: None,
            state: &transcript
          }
          .render(80),
          [],
        );
      }
    }

    case(Reasoning::new(""));
    case(Reasoning::encrypted("foo"));
  }

  #[test]
  fn render_empty_transcript() {
    let transcript = Transcript::default();

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      []
    );
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::from([Span::styled("Thinking...", Style::Muted,)]),
        LineComponent::blank(),
        LineComponent::raw("baz"),
      ]
    );
  }

  #[test]
  fn render_error_entry() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Error("foo".into())]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
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
      ]
    );
  }

  #[test]
  fn render_expanded_draft_reasoning() {
    let mut run = Run::new(0);

    run.update_many(&[
      MessageUpdate::ReasoningDelta {
        index: 0,
        delta: "foo\nbar".into(),
      },
      MessageUpdate::Text {
        index: 1,
        delta: "baz".into(),
      },
    ]);

    let expected = [
      LineComponent::from([Span::styled("Thinking...", Style::Muted)]),
      LineComponent::from([
        Span::styled("  │ ", Style::Muted),
        Span::raw("foo"),
      ]),
      LineComponent::from([
        Span::styled("  │ ", Style::Muted),
        Span::raw("bar"),
      ]),
      LineComponent::blank(),
      LineComponent::raw("baz"),
    ];

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: true,
        run: Some(&run),
        state: &Transcript::default()
      }
      .render(80),
      expected,
    );

    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Draft(run.message)]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: true,
        run: None,
        state: &transcript
      }
      .render(80),
      expected,
    );
  }

  #[test]
  fn render_interrupted_entry() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Interrupted]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [LineComponent::from([Span::styled(
        "■ Conversation interrupted, tell the model what to do differently.",
        Style::Danger,
      )])]
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

      let expected = [LineComponent::from([span])];

      assert_eq!(
        TranscriptComponent {
          reasoning_expanded: false,
          run: Some(&run),
          state: &Transcript::default()
        }
        .render(80),
        expected,
      );

      for entry in [
        TranscriptEntry::Draft(run.message),
        TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
          text,
        )])),
      ] {
        assert_eq!(
          TranscriptComponent {
            reasoning_expanded: false,
            run: None,
            state: &Transcript::with_entries(vec![entry])
          }
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
      "\nfoo\n\n\nbar\n\n".into(),
    )]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [
        LineComponent::blank(),
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::blank(),
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
      TranscriptComponent {
        reasoning_expanded: true,
        run: None,
        state: &transcript
      }
      .render(80),
      [
        LineComponent::from([Span::styled("Thinking...", Style::Muted,)]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("bar"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("qux"),
        ]),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
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
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::Text("foo".into()),
        UserMessageContent::Text("bar".into()),
      ])),
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "baz".into(),
      )])),
      TranscriptEntry::Message(Message::agent(vec![
        AssistantContent::ToolCall(invocation.protocol.clone()),
      ])),
      TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: invocation.protocol.id.clone(),
          name: invocation.protocol.function.name.clone(),
          result: ToolResult {
            outcome: ToolOutcome::Success,
            ..Default::default()
          },
        },
      ])),
    ]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
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
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("●", Style::Success),
          Span::raw(" "),
          Span::raw("Ran rg --files"),
        ]),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(10),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [LineComponent::from([
        Span::styled("●", Style::Accent),
        Span::raw(" "),
        Span::raw("Running rg --files"),
      ])]
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
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
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(80),
      [LineComponent::raw("foo {\"bar\":\"baz\"}")]
    );
  }

  #[test]
  fn render_user_entry_uses_width() {
    let transcript = Transcript::with_entries(vec![TranscriptEntry::Message(
      Message::User(vec![UserMessageContent::Text("foobar\nbaz".into())]),
    )]);

    assert_eq!(
      TranscriptComponent {
        reasoning_expanded: false,
        run: None,
        state: &transcript
      }
      .render(5),
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
