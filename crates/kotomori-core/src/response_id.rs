use super::*;

#[derive(
  Clone,
  Copy,
  Debug,
  Deserialize,
  Eq,
  Hash,
  Ord,
  PartialEq,
  PartialOrd,
  Serialize,
)]
pub struct ResponseId {
  pub round: usize,
  pub run: RunId,
}
