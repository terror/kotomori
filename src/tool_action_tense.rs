#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolActionTense {
  AwaitingApproval,
  Completed,
  Denied,
  Failed,
  Interrupted,
  Progressive,
  Queued,
  Skipped,
}
