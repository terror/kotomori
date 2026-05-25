use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutput {
  pub(crate) status: Option<i32>,
  pub(crate) stderr: String,
  pub(crate) stdout: String,
  pub(crate) success: bool,
}

impl From<Output> for CommandOutput {
  fn from(output: Output) -> Self {
    Self {
      status: output.status.code(),
      stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
      stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
      success: output.status.success(),
    }
  }
}
