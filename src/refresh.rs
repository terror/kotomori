#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refresh {
  Append { from: usize },
  FullAppend,
  Initial,
  RedrawTail { from: usize },
}
