use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunOutcome {
  Cancelled,
  Completed,
  Failed(String),
  Limited(RunLimit),
}
