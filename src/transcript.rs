use super::*;

#[derive(Debug, Default, PartialEq)]
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

  pub(crate) fn interrupt_pending_calls(&mut self) {
    let Some((index, TranscriptEntry::Message(Message::Agent(message)))) =
      self.entries.iter().enumerate().rfind(|(_, entry)| {
        matches!(
          entry,
          TranscriptEntry::Draft(_)
            | TranscriptEntry::Message(Message::Agent(_))
        )
      })
    else {
      return;
    };

    self.entries.extend(
      message
        .content
        .iter()
        .filter_map(|content| match content {
          AssistantContent::ToolCall(call) => Some(call),
          _ => None,
        })
        .filter(|call| {
          !self.entries[index + 1..].iter().any(|entry| match entry {
            TranscriptEntry::Message(Message::User(content)) => content
              .iter()
              .any(|content| content.is_tool_result_for(&call.id)),
            _ => false,
          })
        })
        .map(|call| {
          TranscriptEntry::Message(Message::User(vec![
            UserMessageContent::ToolResult {
              call: call.id.clone(),
              name: call.function.name.clone(),
              result: ToolResult {
                stderr: Some("interrupted".into()),
                ..Default::default()
              },
            },
          ]))
        })
        .collect::<Vec<_>>(),
    );
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  pub(crate) fn last_response(&self) -> Option<String> {
    self.entries.iter().rev().find_map(|entry| match entry {
      TranscriptEntry::Draft(buffer) => buffer.message().text(),
      TranscriptEntry::Message(Message::Agent(message)) => message.text(),
      _ => None,
    })
  }

  pub(crate) fn messages(&self) -> Vec<Message> {
    self
      .entries
      .iter()
      .filter_map(|entry| match entry {
        TranscriptEntry::Draft(buffer) => {
          let message = buffer.message();

          (!message.content.is_empty()).then_some(Message::Agent(message))
        }
        TranscriptEntry::Message(message) => Some(message.clone()),
        TranscriptEntry::Error(_)
        | TranscriptEntry::Interrupted
        | TranscriptEntry::Notice(_) => None,
      })
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
  fn messages_preserve_completed_content_and_boundaries() {
    let messages = vec![
      Message::User(vec![UserMessageContent::Text("foo".into())]),
      Message::agent(vec![
        AssistantContent::text("bar"),
        AssistantContent::ToolCall(
          ToolInvocation::new(
            "foo",
            ToolInvocationKind::Command(CommandTool {
              command: "bar".into(),
              cwd: None,
            }),
          )
          .protocol,
        ),
        AssistantContent::text("baz"),
      ]),
      Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::from_wire("foo"),
        name: ToolName::new("command").unwrap(),
        result: ToolResult::default(),
      }]),
      Message::agent(vec![AssistantContent::reasoning("foo", "qux")]),
      Message::agent(vec![AssistantContent::text("quux")]),
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

  #[test]
  fn messages_skip_unfinished_reasoning() {
    let mut buffer = MessageBuffer::default();

    buffer.apply(MessageUpdate::ReasoningDelta {
      delta: "foo".into(),
      index: 0,
    });

    assert_eq!(
      Transcript::with_entries(vec![TranscriptEntry::Draft(buffer)]).messages(),
      Vec::new()
    );
  }
}
