use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExecutionLimit {
  pub(crate) output_limit: usize,
  pub(crate) timeout: Duration,
  pub(crate) truncated_marker: &'static str,
}

impl Default for ExecutionLimit {
  fn default() -> Self {
    Self {
      output_limit: 20 * 1024,
      timeout: Duration::from_secs(30),
      truncated_marker: "\n[truncated]",
    }
  }
}
