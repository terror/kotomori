use super::*;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Transcript {
  pub(crate) entries: Vec<TranscriptEntry>,
}

impl Transcript {
  pub(crate) fn clear(&mut self) {
    self.entries.clear();
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  pub(crate) fn new(entries: Vec<TranscriptEntry>) -> Self {
    Self { entries }
  }

  pub(crate) fn push(&mut self, entry: TranscriptEntry) {
    self.entries.push(entry);
  }

  pub(crate) fn push_many(
    &mut self,
    entries: impl IntoIterator<Item = TranscriptEntry>,
  ) {
    self.entries.extend(entries);
  }
}
