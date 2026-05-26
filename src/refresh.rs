#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refresh {
  Append { from: usize },
  Initial,
  RedrawScreen,
  RedrawTail { from: usize },
}
