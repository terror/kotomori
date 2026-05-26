use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ToolResult {
  pub(crate) content: Option<String>,
  pub(crate) error: Option<String>,
  pub(crate) exit_status: Option<i32>,
  pub(crate) stdout: Option<String>,
}

impl ToolResult {
  pub(crate) fn content(content: String) -> Self {
    Self {
      content: Some(content),
      error: None,
      exit_status: None,
      stdout: None,
    }
  }

  pub(crate) fn error(error: &impl Display) -> Self {
    Self {
      content: None,
      error: Some(error.to_string()),
      exit_status: None,
      stdout: None,
    }
  }

  fn from_output(output: &Output) -> Self {
    let error = String::from_utf8_lossy(&output.stderr).into_owned();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    Self {
      content: None,
      error: (!error.is_empty()).then_some(error),
      exit_status: output.status.code(),
      stdout: (!stdout.is_empty()).then_some(stdout),
    }
  }

  pub(crate) fn is_error(&self) -> bool {
    self.error.is_some() || self.exit_status.is_some_and(|status| status != 0)
  }

  pub(crate) fn message_content(&self) -> String {
    serde_json::to_string(self).expect("failed to serialize tool result")
  }

  pub(crate) fn output(result: io::Result<Output>) -> Self {
    match result {
      Ok(output) => Self::from_output(&output),
      Err(error) => Self::error(&error),
    }
  }
}
