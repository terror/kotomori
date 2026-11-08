use super::*;

#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimit {
  pub output_limit: usize,
  pub timeout: Duration,
  pub truncated_marker: &'static str,
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
