use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunLimit {
  OutputTokens,
  Rounds { max: usize },
}

impl Display for RunLimit {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::OutputTokens => {
        write!(f, "completion truncated: output token limit reached")
      }
      Self::Rounds { max } => write!(
        f,
        "agent round limit reached: maximum of {max} provider requests per run"
      ),
    }
  }
}
