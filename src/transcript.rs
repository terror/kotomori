use super::*;

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct Transcript {
  pub(crate) entries: Vec<TranscriptEntry>,
}

impl Transcript {
  pub(crate) fn clear(&mut self) {
    self.entries.clear();
  }

  pub(crate) fn error(&mut self, error: String) {
    self.entries.push(TranscriptEntry::Error(error));
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.entries.is_empty()
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

  pub(crate) fn push_message(&mut self, message: Message) {
    self.entries.push(TranscriptEntry::Message(message));
  }

  pub(crate) fn send(&mut self, input: String) {
    self.push_message(Message::User(vec![UserMessageContent::Text(input)]));
  }

  pub(crate) fn with_entries(entries: Vec<TranscriptEntry>) -> Self {
    Self { entries }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clear_removes_history() {
    let mut transcript = Transcript::default();

    assert!(transcript.is_empty());

    transcript.send("foo".into());

    assert!(!transcript.is_empty());

    transcript.clear();

    assert_eq!(transcript, Transcript::default());
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
  fn messages_skip_display_entries() {
    let mut transcript = Transcript::default();

    transcript.send("foo".into());
    transcript.error("bar".into());
    transcript.entries.push(TranscriptEntry::Interrupted);
    transcript.notice("baz");

    assert_eq!(
      transcript.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }
}
