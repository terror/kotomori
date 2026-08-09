use super::*;

#[derive(Debug, Default)]
pub(crate) struct Transcript {
  pub(crate) active_agent_activity: AgentActivity,
  pub(crate) active_elapsed: Duration,
  pub(crate) active_frame: usize,
  pub(crate) entries: Vec<TranscriptEntry>,
}

impl Transcript {
  pub(crate) fn clear(&mut self) {
    self.active_agent_activity = AgentActivity::Idle;
    self.active_elapsed = Duration::ZERO;
    self.entries.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.active_agent_activity = AgentActivity::Idle;
    self.active_elapsed = Duration::ZERO;
    self.entries.push(TranscriptEntry::Error(error));
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
        TranscriptEntry::Error(_)
        | TranscriptEntry::Interrupted
        | TranscriptEntry::Notice(_) => {}
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
            tool_results.push(Message::User(vec![
              UserMessageContent::ToolResult {
                id: invocation.id.clone(),
                result: result.clone(),
              },
            ]));
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

  pub(crate) fn notice(&mut self, notice: impl Into<String>) {
    self.entries.push(TranscriptEntry::Notice(notice.into()));
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clear_removes_entries_and_stops_agent() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.tick(Duration::from_secs(1));
    transcript.clear();

    assert!(transcript.entries.is_empty());

    assert_eq!(transcript.active_elapsed, Duration::ZERO);

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn error_clears_active_activity() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.error("bar".into());

    assert!(!transcript.is_agent_active());

    assert_eq!(transcript.active_elapsed, Duration::ZERO);

    assert!(transcript.messages().is_empty());

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Error(error)] if error == "bar"
    );
  }

  #[test]
  fn finish_agent_activity_ignores_empty_reasoning() {
    let mut transcript = Transcript {
      active_agent_activity: AgentActivity::Reasoning(String::new()),
      ..Default::default()
    };

    transcript.finish_agent_activity();

    assert!(transcript.entries.is_empty());

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn finish_agent_activity_ignores_empty_streaming() {
    let mut transcript = Transcript {
      active_agent_activity: AgentActivity::Streaming(String::new()),
      ..Default::default()
    };

    transcript.finish_agent_activity();

    assert!(transcript.entries.is_empty());

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn finish_agent_activity_ignores_idle() {
    let mut transcript = Transcript::default();

    transcript.finish_agent_activity();

    assert!(transcript.entries.is_empty());

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn finish_agent_activity_ignores_waiting() {
    let mut transcript = Transcript {
      active_agent_activity: AgentActivity::Waiting,
      ..Default::default()
    };

    transcript.finish_agent_activity();

    assert!(transcript.entries.is_empty());

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn finish_agent_activity_preserves_reasoning() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.finish_agent_activity();

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Reasoning(reasoning)] if reasoning == "foo"
    );

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn finish_agent_activity_preserves_streaming() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.finish_agent_activity();

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Agent(message)] if message == "foo"
    );

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Idle);
  }

  #[test]
  fn interrupt_ignores_interrupted_entries_in_messages() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.interrupt();

    assert_eq!(
      transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[test]
  fn interrupt_preserves_active_message() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.tick(Duration::from_secs(1));
    transcript.interrupt();

    assert_eq!(transcript.active_elapsed, Duration::ZERO);

    assert!(!transcript.is_agent_active());

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Agent(message), TranscriptEntry::Interrupted]
        if message == "foo"
    );
  }

  #[test]
  fn is_agent_active_tracks_activity() {
    let mut transcript = Transcript::default();

    assert!(!transcript.is_agent_active());

    transcript.send("foo".into());

    assert!(transcript.is_agent_active());

    transcript.finish_agent_activity();

    assert!(!transcript.is_agent_active());
  }

  #[test]
  fn is_empty_tracks_entries_and_activity() {
    let mut transcript = Transcript::default();

    assert!(transcript.is_empty());

    transcript.send("foo".into());

    assert!(!transcript.is_empty());

    transcript.clear();

    assert!(transcript.is_empty());
  }

  #[test]
  fn messages_flushes_agent_content_before_user() {
    let mut transcript =
      Transcript::with_entries(vec![TranscriptEntry::Agent("foo".into())]);

    transcript.send("bar".into());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![AgentMessageContent::Text("foo".into())]),
        Message::User(vec![UserMessageContent::Text("bar".into())]),
      ]
    );
  }

  #[test]
  fn messages_flushes_tool_results_before_user() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz bar".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(invocation.clone());

    let result = ToolResult {
      content: Some("bar".into()),
      ..Default::default()
    };

    transcript.push_tool_result("foo", result.clone());
    transcript.send("baz".into());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![AgentMessageContent::ToolCall(invocation)]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "foo".into(),
          result,
        }]),
        Message::User(vec![UserMessageContent::Text("baz".into())]),
      ]
    );
  }

  #[test]
  fn messages_ignores_active_agent_activity() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.push_agent_delta("bar");

    assert_eq!(
      transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[test]
  fn messages_ignores_interrupted_entries() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Interrupted]);

    assert!(transcript.messages().is_empty());
  }

  #[test]
  fn messages_ignores_notice_entries() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Notice("foo".into())]);

    assert!(transcript.messages().is_empty());
  }

  #[test]
  fn messages_includes_agent_entries() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::Agent("foo".into())]);

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![AgentMessageContent::Text(
        "foo".into()
      )])]
    );
  }

  #[test]
  fn messages_includes_pending_tool_call() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz bar".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(invocation.clone());

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![AgentMessageContent::ToolCall(
        invocation
      )])]
    );
  }

  #[test]
  fn messages_includes_reasoning_entries() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.finish_agent_activity();

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![AgentMessageContent::Reasoning(
        "foo".into()
      )])]
    );
  }

  #[test]
  fn messages_includes_tool_results() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz bar".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(invocation.clone());

    let result = ToolResult {
      exit_status: Some(0),
      outcome: ToolOutcome::Success,
      stdout: Some("bar\n".into()),
      ..Default::default()
    };

    transcript.push_tool_result("foo", result.clone());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![AgentMessageContent::ToolCall(invocation)]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "foo".into(),
          result,
        }]),
      ]
    );
  }

  #[test]
  fn messages_includes_user_entries() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());

    assert_eq!(
      transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[test]
  fn messages_keeps_adjacent_tool_calls_in_one_agent_message() {
    let mut transcript = Transcript::default();

    let foo = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz bar".into(),
        cwd: None,
      }),
    };

    let bar = ToolInvocation {
      id: "bar".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "qux baz".into(),
        cwd: None,
      }),
    };

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_tool_call(foo.clone());
    transcript.push_tool_call(bar.clone());

    let foo_result = ToolResult {
      content: Some("bar".into()),
      ..Default::default()
    };

    let bar_result = ToolResult {
      content: Some("baz".into()),
      ..Default::default()
    };

    transcript.push_tool_result("foo", foo_result.clone());
    transcript.push_tool_result("bar", bar_result.clone());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![
          AgentMessageContent::Reasoning("foo".into()),
          AgentMessageContent::ToolCall(foo),
          AgentMessageContent::ToolCall(bar),
        ]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "foo".into(),
          result: foo_result,
        }]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "bar".into(),
          result: bar_result,
        }]),
      ]
    );
  }

  #[test]
  fn messages_preserves_reasoning_with_tool_calls() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz bar".into(),
        cwd: None,
      }),
    };

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_tool_call(invocation.clone());

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![
        AgentMessageContent::Reasoning("foo".into()),
        AgentMessageContent::ToolCall(invocation),
      ])]
    );
  }

  #[test]
  fn notice_preserves_active_activity() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.notice("bar");

    assert!(transcript.is_agent_active());

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Notice(notice)] if notice == "bar"
    );
  }

  #[test]
  fn push_agent_delta_appends_to_streaming() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.push_agent_delta("bar");
    transcript.finish_agent_activity();

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![AgentMessageContent::Text(
        "foobar".into()
      )])]
    );
  }

  #[test]
  fn push_agent_delta_empty_sets_waiting() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("");

    assert!(transcript.is_agent_active());

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);
  }

  #[test]
  fn push_agent_delta_preserves_reasoning_before_streaming() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_delta("bar");
    transcript.finish_agent_activity();

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![
        AgentMessageContent::Reasoning("foo".into()),
        AgentMessageContent::Text("bar".into()),
      ])]
    );
  }

  #[test]
  fn push_agent_delta_preserves_reasoning_before_waiting() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_delta("");

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Reasoning(reasoning)] if reasoning == "foo"
    );

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);
  }

  #[test]
  fn push_agent_reasoning_delta_appends_to_reasoning() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_reasoning_delta("bar");
    transcript.finish_agent_activity();

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![AgentMessageContent::Reasoning(
        "foobar".into()
      )])]
    );
  }

  #[test]
  fn push_agent_reasoning_delta_empty_sets_waiting() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("");

    assert!(transcript.is_agent_active());

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);
  }

  #[test]
  fn push_agent_reasoning_delta_preserves_streaming_before_reasoning() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.push_agent_reasoning_delta("bar");
    transcript.finish_agent_activity();

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![
        AgentMessageContent::Text("foo".into()),
        AgentMessageContent::Reasoning("bar".into()),
      ])]
    );
  }

  #[test]
  fn push_agent_reasoning_delta_preserves_streaming_before_waiting() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.push_agent_reasoning_delta("");

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Agent(message)] if message == "foo"
    );

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);
  }

  #[test]
  fn push_tool_call_preserves_active_message() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(invocation.clone());

    assert_eq!(
      transcript.messages(),
      vec![Message::Agent(vec![
        AgentMessageContent::Text("foo".into()),
        AgentMessageContent::ToolCall(invocation),
      ])]
    );

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);
  }

  #[test]
  fn push_tool_result_ignores_unknown_id() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(invocation);
    transcript.push_tool_result(
      "bar",
      ToolResult {
        content: Some("baz".into()),
        ..Default::default()
      },
    );

    assert_matches!(
      &transcript.entries[..],
      [TranscriptEntry::Tool { result: None, .. }]
    );

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);
  }

  #[test]
  fn push_tool_result_updates_latest_matching_tool_call() {
    let mut transcript = Transcript::default();

    let foo = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    let bar = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(foo);
    transcript.push_tool_call(bar);

    let result = ToolResult {
      content: Some("qux".into()),
      ..Default::default()
    };

    transcript.push_tool_result("foo", result.clone());

    assert_matches!(
      &transcript.entries[..],
      [
        TranscriptEntry::Tool { result: None, .. },
        TranscriptEntry::Tool {
          result: Some(entry_result),
          ..
        },
      ] if entry_result == &result
    );
  }

  #[test]
  fn push_tool_result_updates_matching_tool_call() {
    let mut transcript = Transcript::default();

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    transcript.push_tool_call(invocation.clone());

    let result = ToolResult {
      content: Some("bar".into()),
      ..Default::default()
    };

    transcript.push_tool_result("foo", result.clone());

    assert_eq!(
      transcript.messages(),
      vec![
        Message::Agent(vec![AgentMessageContent::ToolCall(invocation)]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "foo".into(),
          result,
        }]),
      ]
    );
  }

  #[test]
  fn send_resets_active_activity() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.tick(Duration::from_secs(1));
    transcript.send("bar".into());

    assert_eq!(transcript.active_elapsed, Duration::ZERO);

    assert_eq!(transcript.active_frame, 0);

    assert_matches!(&transcript.active_agent_activity, AgentActivity::Waiting);

    assert_eq!(
      transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("bar".into())])]
    );
  }

  #[test]
  fn tick_advances_active_activity() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.tick(Duration::from_secs(1));

    assert_eq!(transcript.active_elapsed, Duration::from_secs(1));
    assert_eq!(transcript.active_frame, 1);
  }

  #[test]
  fn tick_ignores_idle_activity() {
    let mut transcript = Transcript::default();

    transcript.tick(Duration::from_secs(1));

    assert_eq!(transcript.active_elapsed, Duration::ZERO);
    assert_eq!(transcript.active_frame, 0);
  }

  #[test]
  fn tick_saturates_active_elapsed() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.active_elapsed = Duration::MAX;
    transcript.active_frame = usize::MAX;
    transcript.tick(Duration::from_secs(1));

    assert_eq!(transcript.active_elapsed, Duration::MAX);
    assert_eq!(transcript.active_frame, 0);
  }

  #[test]
  fn with_entries_uses_entries() {
    let transcript =
      Transcript::with_entries(vec![TranscriptEntry::User("foo".into())]);

    assert_eq!(
      transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );

    assert!(!transcript.is_agent_active());
  }
}
