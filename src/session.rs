use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct Session {
  pub(crate) core: kotomori_core::Session,
  pub(crate) created_at: u64,
  pub(crate) id: Option<i64>,
  pub(crate) settings: Settings,
  pub(crate) title: Option<String>,
  pub(crate) transcript: Vec<TranscriptEntry>,
  pub(crate) updated_at: u64,
}

impl Session {
  const TITLE_LENGTH: usize = 80;

  pub(crate) fn clear(&mut self) {
    self.core.clear();
    self.transcript.clear();
  }

  pub(crate) fn conversation(&self) -> &Conversation {
    self.core.conversation()
  }

  pub(crate) fn generate_title(&self) -> Option<String> {
    self
      .conversation()
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

  pub(crate) fn interrupt_pending_calls(&mut self) -> Result {
    self.core.interrupt_pending_calls()?;
    self.refresh();

    Ok(())
  }

  pub(crate) fn new(settings: &Settings, now: u64) -> Self {
    Self {
      core: kotomori_core::Session::default(),
      created_at: now,
      id: None,
      settings: settings.clone(),
      title: None,
      transcript: Vec::new(),
      updated_at: now,
    }
  }

  pub(crate) fn push(&mut self, entry: TranscriptEntry) {
    self.core.append(entry.into_session_entry());
    self.refresh();
  }

  #[cfg(test)]
  pub(crate) fn push_many(
    &mut self,
    entries: impl IntoIterator<Item = TranscriptEntry>,
  ) {
    for entry in entries {
      self.push(entry);
    }
  }

  pub(crate) fn refresh(&mut self) {
    self.transcript = self
      .core
      .entries()
      .iter()
      .filter_map(TranscriptEntry::from_session_entry)
      .collect();
  }

  pub(crate) fn snapshot(&self) -> Result<SessionSnapshot> {
    self.core.snapshot()
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
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
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
      session.conversation().messages(),
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

    let mut session = Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:foo".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    );

    session.push(TranscriptEntry::Draft(buffer));

    assert_eq!(session.conversation().messages(), []);
  }
}
