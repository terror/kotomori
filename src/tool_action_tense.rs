#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolActionTense {
  Completed,
  Denied,
  Failed,
  Interrupted,
  Progressive,
  Skipped,
}
