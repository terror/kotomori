#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
  RunAgent { input: String },
}
