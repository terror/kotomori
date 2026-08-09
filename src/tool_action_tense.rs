#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolActionTense {
  Completed,
  Failed,
  Progressive,
}
