#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolActionTense {
  Completed,
  Failed,
  Progressive,
}
