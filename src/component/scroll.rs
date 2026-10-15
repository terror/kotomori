use super::*;

#[derive(Debug)]
#[must_use]
pub(crate) struct ScrollComponent {
  pub(crate) height: usize,
  pub(crate) offset: usize,
  pub(crate) rows: Vec<LineComponent>,
}

impl ScrollComponent {
  pub(crate) fn layout(self) -> Vec<LineComponent> {
    self
      .rows
      .into_iter()
      .skip(self.offset)
      .take(self.height)
      .collect()
  }
}
