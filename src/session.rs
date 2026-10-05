use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct Session {
  pub(crate) created_at: u64,
  pub(crate) id: Option<i64>,
  pub(crate) settings: Settings,
  pub(crate) title: Option<String>,
  pub(crate) transcript: Transcript,
  pub(crate) updated_at: u64,
}

impl Session {
  const TITLE_LENGTH: usize = 80;

  pub(crate) fn generate_title(&self) -> Option<String> {
    self
      .transcript
      .entries
      .iter()
      .filter_map(TranscriptEntry::message)
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

  pub(crate) fn new(settings: &Settings, now: u64) -> Self {
    Self {
      created_at: now,
      id: None,
      settings: settings.clone(),
      title: None,
      transcript: Transcript::default(),
      updated_at: now,
    }
  }
}
