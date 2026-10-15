#[derive(Debug, Default)]
pub(crate) struct Selection {
  index: usize,
}

impl Selection {
  pub(crate) fn clamp(&mut self, len: usize) {
    self.index = self.index(len).unwrap_or_default();
  }

  pub(crate) fn index(&self, len: usize) -> Option<usize> {
    len.checked_sub(1).map(|last| self.index.min(last))
  }

  pub(crate) fn select_next(&mut self, len: usize) {
    self.index = self.index(len).map_or(0, |index| (index + 1) % len);
  }

  pub(crate) fn select_previous(&mut self, len: usize) {
    self.index = match self.index(len) {
      Some(0) => len - 1,
      Some(index) => index - 1,
      None => 0,
    };
  }
}
