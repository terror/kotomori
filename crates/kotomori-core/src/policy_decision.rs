#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyDecision {
  Allow,
  Ask,
  Deny { reason: String },
}
