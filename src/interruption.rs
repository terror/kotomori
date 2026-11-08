#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Interruption {
  Clear,
  ContinueQueued,
  Error(String),
  Stop,
}
