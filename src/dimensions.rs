#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Dimensions {
  pub(crate) height: u16,
  pub(crate) width: u16,
}

impl Dimensions {
  pub(crate) fn height(self) -> usize {
    usize::from(self.height)
  }
}
