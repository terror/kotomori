use super::*;

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct Transcript {
  pub(crate) active_agent_activity: AgentActivity,
  pub(crate) active_content: Vec<AgentMessageContent>,
  pub(crate) active_elapsed: Duration,
  pub(crate) active_frame: usize,
  pub(crate) entries: Vec<TranscriptEntry>,
}

impl Transcript {
  pub(crate) fn clear(&mut self) {
    self.active_agent_activity = AgentActivity::Idle;
    self.active_elapsed = Duration::ZERO;
    self.active_content.clear();
    self.entries.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.finish_agent_activity();
    self.active_elapsed = Duration::ZERO;
    self.entries.push(TranscriptEntry::Error(error));
  }

  pub(crate) fn finish_agent_activity(&mut self) {
    match mem::take(&mut self.active_agent_activity) {
      AgentActivity::Reasoning(reasoning) if !reasoning.is_empty() => {
        self
          .active_content
          .push(AgentMessageContent::Reasoning(reasoning));
      }
      AgentActivity::Streaming(message) if !message.is_empty() => {
        self.active_content.push(AgentMessageContent::Text(message));
      }
      AgentActivity::Idle
      | AgentActivity::Reasoning(_)
      | AgentActivity::Streaming(_)
      | AgentActivity::Waiting => {}
    }

    if !self.active_content.is_empty() {
      self
        .entries
        .push(TranscriptEntry::Message(Message::Agent(mem::take(
          &mut self.active_content,
        ))));
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
    self
      .entries
      .iter()
      .filter_map(TranscriptEntry::message)
      .cloned()
      .collect()
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
            self
              .active_content
              .push(AgentMessageContent::Reasoning(reasoning));
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
            self.active_content.push(AgentMessageContent::Text(message));
          }

          if delta.is_empty() {
            AgentActivity::Waiting
          } else {
            AgentActivity::Reasoning(delta.into())
          }
        }
      };
  }

  pub(crate) fn push_message(&mut self, message: Message) {
    self.active_content.clear();
    self.active_agent_activity = AgentActivity::Waiting;
    self.entries.push(TranscriptEntry::Message(message));
  }

  pub(crate) fn send(&mut self, input: String) {
    self.active_agent_activity = AgentActivity::Waiting;
    self.active_elapsed = Duration::ZERO;
    self.active_frame = 0;
    self.active_content.clear();

    self
      .entries
      .push(TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::Text(input),
      ])));
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
  fn clear_removes_history_and_streaming_state() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.push_agent_reasoning_delta("bar");
    transcript.push_agent_delta("baz");
    transcript.tick(Duration::from_secs(1));
    transcript.clear();

    assert_eq!(
      transcript,
      Transcript {
        active_frame: 1,
        ..Default::default()
      }
    );
  }

  #[test]
  fn error_preserves_active_content() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_delta("bar");
    transcript.push_agent_reasoning_delta("baz");
    transcript.tick(Duration::from_secs(1));
    transcript.error("qux".into());

    assert_eq!(
      transcript,
      Transcript {
        active_frame: 1,
        entries: vec![
          TranscriptEntry::Message(Message::Agent(vec![
            AgentMessageContent::Reasoning("foo".into()),
            AgentMessageContent::Text("bar".into()),
            AgentMessageContent::Reasoning("baz".into()),
          ])),
          TranscriptEntry::Error("qux".into()),
        ],
        ..Default::default()
      }
    );
  }

  #[test]
  fn finish_agent_activity_ignores_empty_content() {
    for active_agent_activity in [
      AgentActivity::Idle,
      AgentActivity::Waiting,
      AgentActivity::Reasoning(String::new()),
      AgentActivity::Streaming(String::new()),
    ] {
      let mut transcript = Transcript {
        active_agent_activity,
        ..Default::default()
      };

      transcript.finish_agent_activity();

      assert_eq!(transcript, Transcript::default());
    }
  }

  #[test]
  fn finish_agent_activity_preserves_content_once() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_reasoning_delta("bar");
    transcript.push_agent_delta("baz");
    transcript.push_agent_delta("qux");
    transcript.finish_agent_activity();
    transcript.finish_agent_activity();

    assert_eq!(
      transcript,
      Transcript {
        entries: vec![TranscriptEntry::Message(Message::Agent(vec![
          AgentMessageContent::Reasoning("foobar".into()),
          AgentMessageContent::Text("bazqux".into()),
        ]))],
        ..Default::default()
      }
    );
  }

  #[test]
  fn interrupt_preserves_active_content() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.tick(Duration::from_secs(1));
    transcript.interrupt();

    assert_eq!(
      transcript,
      Transcript {
        active_frame: 1,
        entries: vec![
          TranscriptEntry::Message(Message::Agent(vec![
            AgentMessageContent::Text("foo".into()),
          ])),
          TranscriptEntry::Interrupted,
        ],
        ..Default::default()
      }
    );
  }

  #[test]
  fn is_empty_tracks_history_and_activity() {
    let mut transcript = Transcript::default();

    assert!(transcript.is_empty());

    transcript.push_agent_delta("");

    assert!(!transcript.is_empty());

    transcript.finish_agent_activity();

    assert!(transcript.is_empty());

    transcript.send("foo".into());
    transcript.finish_agent_activity();

    assert!(!transcript.is_empty());
  }

  #[test]
  fn messages_preserve_completed_content_and_boundaries() {
    let messages = vec![
      Message::User(vec![UserMessageContent::Text("foo".into())]),
      Message::Agent(vec![
        AgentMessageContent::Text("bar".into()),
        AgentMessageContent::ToolCall(ToolInvocation {
          id: "foo".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            command: "bar".into(),
            cwd: None,
          }),
        }),
        AgentMessageContent::Text("baz".into()),
      ]),
      Message::User(vec![UserMessageContent::ToolResult {
        id: "foo".into(),
        result: ToolResult::default(),
      }]),
      Message::Agent(vec![AgentMessageContent::Reasoning("qux".into())]),
      Message::Agent(vec![AgentMessageContent::Text("quux".into())]),
    ];

    let transcript = Transcript::with_entries(
      messages
        .iter()
        .cloned()
        .map(TranscriptEntry::Message)
        .collect(),
    );

    assert_eq!(transcript.messages(), messages);
  }

  #[test]
  fn messages_skip_display_entries_and_streaming_content() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.error("bar".into());
    transcript.interrupt();
    transcript.notice("baz");
    transcript.push_agent_reasoning_delta("qux");
    transcript.push_agent_delta("quux");

    assert_eq!(
      transcript.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[test]
  fn notice_preserves_streaming_content() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.notice("bar");

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Streaming("foo".into()),
        entries: vec![TranscriptEntry::Notice("bar".into())],
        ..Default::default()
      }
    );
  }

  #[test]
  fn push_agent_delta_empty_preserves_reasoning() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_delta("");

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Waiting,
        active_content: vec![AgentMessageContent::Reasoning("foo".into())],
        ..Default::default()
      }
    );
  }

  #[test]
  fn push_agent_reasoning_delta_empty_preserves_text() {
    let mut transcript = Transcript::default();

    transcript.push_agent_delta("foo");
    transcript.push_agent_reasoning_delta("");

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Waiting,
        active_content: vec![AgentMessageContent::Text("foo".into())],
        ..Default::default()
      }
    );
  }

  #[test]
  fn push_agent_reasoning_delta_empty_sets_waiting() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("");

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Waiting,
        ..Default::default()
      }
    );
  }

  #[test]
  fn push_message_replaces_streaming_content() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_delta("bar");

    let message = Message::Agent(vec![AgentMessageContent::Text("baz".into())]);

    transcript.push_message(message.clone());

    let expected = Transcript {
      active_agent_activity: AgentActivity::Waiting,
      entries: vec![TranscriptEntry::Message(message)],
      ..Default::default()
    };

    assert_eq!(transcript, expected);

    transcript.finish_agent_activity();

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Idle,
        ..expected
      }
    );
  }

  #[test]
  fn send_resets_streaming_state() {
    let mut transcript = Transcript::default();

    transcript.push_agent_reasoning_delta("foo");
    transcript.push_agent_delta("bar");
    transcript.tick(Duration::from_secs(1));
    transcript.send("baz".into());

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Waiting,
        entries: vec![TranscriptEntry::Message(Message::User(vec![
          UserMessageContent::Text("baz".into())
        ]))],
        ..Default::default()
      }
    );
  }

  #[test]
  fn tick_advances_active_activity() {
    let mut transcript = Transcript {
      active_agent_activity: AgentActivity::Waiting,
      ..Default::default()
    };

    transcript.tick(Duration::from_secs(1));

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Waiting,
        active_elapsed: Duration::from_secs(1),
        active_frame: 1,
        ..Default::default()
      }
    );
  }

  #[test]
  fn tick_ignores_idle_activity() {
    let mut transcript = Transcript::default();

    transcript.tick(Duration::from_secs(1));

    assert_eq!(transcript, Transcript::default());
  }

  #[test]
  fn tick_saturates_elapsed_and_wraps_frame() {
    let mut transcript = Transcript {
      active_agent_activity: AgentActivity::Waiting,
      active_elapsed: Duration::MAX,
      active_frame: usize::MAX,
      ..Default::default()
    };

    transcript.tick(Duration::from_secs(1));

    assert_eq!(
      transcript,
      Transcript {
        active_agent_activity: AgentActivity::Waiting,
        active_elapsed: Duration::MAX,
        ..Default::default()
      }
    );
  }
}
