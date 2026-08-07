use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Frame {
  pub(crate) dimensions: Dimensions,
  pub(crate) lines: Vec<String>,
}

impl Frame {
  pub(crate) fn last_row(&self) -> usize {
    self.lines.len().saturating_sub(1)
  }

  pub(crate) fn len(&self) -> usize {
    self.lines.len()
  }

  pub(crate) fn new(lines: Vec<String>, dimensions: Dimensions) -> Self {
    Self { dimensions, lines }
  }
}
