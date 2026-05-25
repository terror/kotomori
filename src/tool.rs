use super::*;

mod apply_patch;
mod command;
mod list_files;
mod read_file;
mod search_files;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandInvocation {
  pub(crate) arguments: Vec<String>,
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) program: String,
}

impl Display for CommandInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    if self.arguments.is_empty() {
      write!(f, "{}", self.program)
    } else {
      write!(f, "{} {}", self.program, self.arguments.join(" "))
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct CommandOutput {
  pub(crate) status: Option<i32>,
  pub(crate) stderr: String,
  pub(crate) stdout: String,
  pub(crate) success: bool,
}

impl From<std::process::Output> for CommandOutput {
  fn from(output: std::process::Output) -> Self {
    Self {
      status: output.status.code(),
      stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
      stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
      success: output.status.success(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCall {
  pub(crate) arguments: Value,
  pub(crate) id: String,
  pub(crate) name: String,
}

impl ToolCall {
  pub(crate) fn from_arguments_string(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl AsRef<str>,
  ) -> Result<Self> {
    let name = name.into();
    let arguments = serde_json::from_str(arguments.as_ref())
      .with_context(|| format!("failed to parse `{name}` arguments"))?;

    Ok(Self::new(id, name, arguments))
  }

  pub(crate) fn invocation(&self) -> Result<ToolInvocation> {
    tools()
      .into_iter()
      .find(|tool| tool.name == self.name.as_str())
      .with_context(|| format!("unknown tool `{}`", self.name))?
      .invocation(self.clone())
  }

  pub(crate) fn new(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: Value,
  ) -> Self {
    Self {
      arguments,
      id: id.into(),
      name: name.into(),
    }
  }
}

impl Display for ToolCall {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{} {}", self.name, self.arguments)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolAction {
  ApplyPatch { cwd: Option<PathBuf>, patch: String },
  Command(CommandInvocation),
  ReadFile { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ToolError {
  pub(crate) message: String,
}

impl ToolError {
  pub(crate) fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInvocation {
  pub(crate) action: ToolAction,
  pub(crate) call: ToolCall,
}

impl ToolInvocation {
  fn action(&self, tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Past => match self.call.name.as_str() {
        "list_files" => "Listed",
        "read_file" => "Read",
        _ => "Ran",
      },
      ToolActionTense::Progressive => match self.call.name.as_str() {
        "list_files" => "Listing",
        "read_file" => "Reading",
        _ => "Running",
      },
    }
  }

  fn command(&self) -> Option<&CommandInvocation> {
    match &self.action {
      ToolAction::Command(command) => Some(command),
      ToolAction::ApplyPatch { .. } | ToolAction::ReadFile { .. } => None,
    }
  }

  #[allow(dead_code)]
  pub(crate) fn past_tense(&self) -> String {
    self.title(ToolActionTense::Past)
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn subject(&self) -> String {
    match self.call.name.as_str() {
      "apply_patch" => "apply_patch".into(),
      "command" | "search_files" => self
        .command()
        .map_or_else(|| self.call.name.clone(), ToString::to_string),
      "list_files" => self
        .command()
        .and_then(|command| command.cwd.as_ref())
        .map_or_else(
          || "files".into(),
          |cwd| format!("files in {}", cwd.display()),
        ),
      "read_file" => match &self.action {
        ToolAction::ReadFile { path } => path.display().to_string(),
        ToolAction::ApplyPatch { .. } | ToolAction::Command(_) => {
          self.call.name.clone()
        }
      },
      _ => self.call.name.clone(),
    }
  }

  fn title(&self, tense: ToolActionTense) -> String {
    format!("{} {}", self.action(tense), self.subject())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolActionTense {
  Past,
  Progressive,
}

impl Display for ToolInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self.call.name.as_str() {
      "apply_patch" => write!(f, "apply_patch"),
      "command" | "search_files" => {
        if let Some(command) = self.command() {
          write!(f, "{command}")
        } else {
          write!(f, "{}", self.call.name)
        }
      }
      "list_files" => {
        if let Some(cwd) =
          self.command().and_then(|command| command.cwd.as_ref())
        {
          write!(f, "list files in {}", cwd.display())
        } else {
          write!(f, "list files")
        }
      }
      "read_file" => match &self.action {
        ToolAction::ReadFile { path } => write!(f, "read {}", path.display()),
        ToolAction::ApplyPatch { .. } | ToolAction::Command(_) => {
          write!(f, "{}", self.call.name)
        }
      },
      _ => write!(f, "{}", self.call.name),
    }
  }
}

#[derive(Clone, Copy)]
pub(crate) struct RegisteredTool {
  pub(crate) description: &'static str,
  invocation: fn(ToolCall) -> Result<ToolInvocation>,
  pub(crate) name: &'static str,
  parameters: fn() -> Value,
}

impl RegisteredTool {
  pub(crate) fn invocation(&self, call: ToolCall) -> Result<ToolInvocation> {
    (self.invocation)(call)
  }

  fn new<T: Tool>() -> Self {
    Self {
      description: T::DESCRIPTION,
      invocation: T::invocation,
      name: T::NAME,
      parameters: T::parameters,
    }
  }

  pub(crate) fn parameters(&self) -> Value {
    (self.parameters)()
  }
}

pub(crate) trait Tool: serde::de::DeserializeOwned + Sized {
  const DESCRIPTION: &'static str;

  const NAME: &'static str;

  fn action(self) -> ToolAction;

  fn decode_arguments(call: &ToolCall) -> Result<Self> {
    serde_json::from_value(call.arguments.clone())
      .with_context(|| format!("failed to decode `{}` arguments", call.name))
  }

  fn invocation(call: ToolCall) -> Result<ToolInvocation> {
    let tool = Self::decode_arguments(&call)?;

    Ok(ToolInvocation {
      action: tool.action(),
      call,
    })
  }

  fn parameters() -> Value;
}

pub(crate) fn tools() -> Vec<RegisteredTool> {
  vec![
    RegisteredTool::new::<list_files::ListFiles>(),
    RegisteredTool::new::<search_files::SearchFiles>(),
    RegisteredTool::new::<read_file::ReadFile>(),
    RegisteredTool::new::<command::Command>(),
    RegisteredTool::new::<apply_patch::ApplyPatch>(),
  ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ToolOutput {
  Command(CommandOutput),
  Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ToolResult {
  pub(crate) invocation: ToolInvocation,
  pub(crate) output: std::result::Result<ToolOutput, ToolError>,
}

impl ToolResult {
  #[allow(dead_code)]
  pub(crate) fn error(
    invocation: ToolInvocation,
    message: impl Into<String>,
  ) -> Self {
    Self {
      invocation,
      output: Err(ToolError::new(message)),
    }
  }

  #[allow(dead_code)]
  pub(crate) fn message(&self) -> String {
    match &self.output {
      Ok(output) => format!(
        "Tool result for `{}`:\n{}",
        self.invocation,
        output.content()
      ),
      Err(error) => {
        format!("Tool `{}` failed:\n{}", self.invocation, error.message)
      }
    }
  }

  #[allow(dead_code)]
  pub(crate) fn ok(invocation: ToolInvocation, output: ToolOutput) -> Self {
    Self {
      invocation,
      output: Ok(output),
    }
  }
}

impl ToolOutput {
  fn command_content(output: &CommandOutput) -> String {
    let mut content = String::new();

    content.push_str("status: ");
    content.push_str(
      &output
        .status
        .map_or_else(|| "signal".into(), |status| status.to_string()),
    );
    content.push('\n');

    if !output.stdout.is_empty() {
      content.push_str("stdout:\n");
      content.push_str(&output.stdout);
    }

    if !output.stderr.is_empty() {
      content.push_str("stderr:\n");
      content.push_str(&output.stderr);
    }

    content
  }

  pub(crate) fn content(&self) -> String {
    match self {
      Self::Command(output) => Self::command_content(output),
      Self::Text(content) => content.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_apply_patch_tool_call() {
    assert_eq!(
      ToolCall::new("foo", "apply_patch", json!({"patch": "bar"}))
        .invocation()
        .unwrap()
        .action,
      ToolAction::ApplyPatch {
        cwd: None,
        patch: "bar".into(),
      },
    );
  }

  #[test]
  fn parses_command_tool_call() {
    assert_eq!(
      ToolCall::new(
        "foo",
        "command",
        json!({"program": "bar", "arguments": ["baz"], "cwd": null}),
      )
      .invocation()
      .unwrap()
      .action,
      ToolAction::Command(CommandInvocation {
        arguments: vec!["baz".into()],
        cwd: None,
        program: "bar".into(),
      }),
    );
  }

  #[test]
  fn parses_list_files_tool_call() {
    assert_eq!(
      ToolCall::new("foo", "list_files", json!({"cwd": "bar"}))
        .invocation()
        .unwrap()
        .action,
      ToolAction::Command(CommandInvocation {
        arguments: vec!["--files".into()],
        cwd: Some("bar".into()),
        program: "rg".into(),
      }),
    );
  }

  #[test]
  fn parses_read_file_tool_call() {
    assert_eq!(
      ToolCall::new("foo", "read_file", json!({"path": "bar"}))
        .invocation()
        .unwrap()
        .action,
      ToolAction::ReadFile { path: "bar".into() },
    );
  }

  #[test]
  fn parses_search_files_tool_call() {
    assert_eq!(
      ToolCall::new(
        "foo",
        "search_files",
        json!({"arguments": ["foo"], "cwd": "bar"}),
      )
      .invocation()
      .unwrap()
      .action,
      ToolAction::Command(CommandInvocation {
        arguments: vec!["foo".into()],
        cwd: Some("bar".into()),
        program: "rg".into(),
      }),
    );
  }

  #[test]
  fn parses_tool_call_arguments() {
    assert_eq!(
      ToolCall::from_arguments_string("foo", "read_file", r#"{"path":"bar"}"#)
        .unwrap(),
      ToolCall::new("foo", "read_file", json!({"path": "bar"})),
    );
  }

  #[test]
  fn unknown_tool_errors() {
    assert_eq!(
      ToolCall::new("foo", "bar", json!({}))
        .invocation()
        .unwrap_err()
        .to_string(),
      "unknown tool `bar`",
    );
  }
}
