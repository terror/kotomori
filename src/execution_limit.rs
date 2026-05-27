use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExecutionLimit {
  pub(crate) output_limit: usize,
  pub(crate) timeout: Duration,
  pub(crate) truncated_marker: &'static str,
}

impl ExecutionLimit {
  /// Decode collected process output after applying the configured byte limit.
  ///
  /// Callers read one byte past the limit when possible so that truncation can
  /// be detected without buffering unbounded output. When output is truncated,
  /// the retained bytes may be shortened further to keep the marker within the
  /// configured budget.
  pub(crate) fn decode(&self, mut bytes: Vec<u8>) -> String {
    let truncated = bytes.len() > self.output_limit;

    if truncated {
      bytes.truncate(self.output_limit);
    }

    if truncated
      && bytes.len() + self.truncated_marker.len() > self.output_limit
    {
      bytes.truncate(
        self
          .output_limit
          .saturating_sub(self.truncated_marker.len()),
      );
    }

    let mut output = String::from_utf8_lossy(&bytes).into_owned();

    if truncated {
      output.push_str(self.truncated_marker);
    }

    output
  }
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
