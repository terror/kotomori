use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawToolCall {
  pub(crate) arguments: Value,
  pub(crate) id: String,
  pub(crate) name: String,
}

impl Display for RawToolCall {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{} {}", self.name, self.arguments)
  }
}
