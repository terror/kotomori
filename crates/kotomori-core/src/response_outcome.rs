#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponseOutcome {
  Completed,
  Failed(String),
  Interrupted,
}
