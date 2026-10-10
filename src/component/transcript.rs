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
      .push(self.agent_content(TranscriptBlock::draft(&run.message, &[])));

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
    blocks: impl IntoIterator<Item = TranscriptBlock<'a>>,
  ) -> StackComponent<'a> {
    blocks
      .into_iter()
      .fold(
        StackComponent::default().gap(1),
        |stack, block| match block {
          TranscriptBlock::Markdown(text) => {
            stack.push(MarkdownComponent { text })
          }
          TranscriptBlock::Raw(text) => stack.push(LineComponent::raw(text)),
          TranscriptBlock::Reasoning(reasoning) => {
            if reasoning.is_empty() {
              return stack;
            }

            let component =
              StackComponent::default().push(LineComponent::from([
                Span::styled("Thinking...", Style::Muted),
              ]));

            stack.push(if self.reasoning_expanded {
              component.push(
                LinesComponent::raw(reasoning.lines())
                  .gutter(Span::styled("  │ ", Style::Muted)),
              )
            } else {
              component
            })
          }
          TranscriptBlock::Tool { invocation, result } => {
            stack.push(TranscriptToolInvocationComponent { invocation, result })
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
              TranscriptEntry::Draft(buffer) => {
                stack.push(self.agent_content(TranscriptBlock::draft(
                  buffer,
                  &self.state.entries[index + offset + 1..],
                )))
              }
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
                stack.push(self.agent_content(TranscriptBlock::content(
                  &message.content,
                  &self.state.entries[index + offset + 1..],
                )))
              }
              TranscriptEntry::Message(Message::User(content)) => {
                content.iter().filter_map(UserMessageContent::text).fold(
                  stack,
                  |stack, text| {
                    stack.push(
                      LinesComponent::raw(text.split('\n'))
                        .gutter(Span::styled("│ ", Style::Accent)),
                    )
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
}
