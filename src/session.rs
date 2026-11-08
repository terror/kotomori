use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct Session {
  pub(crate) conversation: Conversation,
  pub(crate) created_at: u64,
  pub(crate) id: Option<i64>,
  pub(crate) settings: Settings,
  pub(crate) title: Option<String>,
  pub(crate) transcript: Transcript,
  pub(crate) updated_at: u64,
}

impl Session {
  const TITLE_LENGTH: usize = 80;

  pub(crate) fn clear(&mut self) {
    self.conversation.clear();
    self.transcript.clear();
  }

  pub(crate) fn commit(
    &mut self,
    message: Message,
    draft: Option<MessageBuffer>,
  ) {
    let entry = match draft {
      Some(draft) => TranscriptEntry::Draft(draft),
      None => TranscriptEntry::Message(message.clone()),
    };

    self.transcript.push_many(
      self
        .conversation
        .push(message)
        .into_iter()
        .map(TranscriptEntry::Message),
    );

    self.transcript.push(entry);
  }

  pub(crate) fn generate_title(&self) -> Option<String> {
    self
      .conversation
      .messages()
      .iter()
      .filter_map(Message::user_content)
      .find_map(|content| {
        let title = content
          .split_whitespace()
          .collect::<Vec<_>>()
          .join(" ")
          .as_str()
          .truncate(Self::TITLE_LENGTH);

        (!title.is_empty()).then_some(title)
      })
  }

  pub(crate) fn interrupt_pending_calls(&mut self) {
    self.transcript.push_many(
      self
        .conversation
        .interrupt_pending_calls()
        .into_iter()
        .map(TranscriptEntry::Message),
    );
  }

  pub(crate) fn new(settings: &Settings, now: u64) -> Self {
    Self {
      conversation: Conversation::default(),
      created_at: now,
      id: None,
      settings: settings.clone(),
      title: None,
      transcript: Transcript::default(),
      updated_at: now,
    }
  }

  pub(crate) fn push(&mut self, entry: TranscriptEntry) {
    let messages = match &entry {
      TranscriptEntry::Message(message) => {
        self.conversation.push(message.clone())
      }
      TranscriptEntry::Draft(buffer) => self.conversation.push_partial(buffer),
      TranscriptEntry::Error(_)
      | TranscriptEntry::Interrupted
      | TranscriptEntry::Notice(_) => Vec::new(),
    };

    self
      .transcript
      .push_many(messages.into_iter().map(TranscriptEntry::Message));

    self.transcript.push(entry);
  }

  pub(crate) fn push_many(
    &mut self,
    entries: impl IntoIterator<Item = TranscriptEntry>,
  ) {
    for entry in entries {
      self.push(entry);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn messages_skip_display_entries() {
    let mut session = Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:foo".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    );

    session.push_many([
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "foo".into(),
      )])),
      TranscriptEntry::Error("bar".into()),
      TranscriptEntry::Interrupted,
      TranscriptEntry::Notice("baz".into()),
    ]);

    assert_eq!(
      session.conversation.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }
}
