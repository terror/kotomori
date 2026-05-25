use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub(crate) struct ToolCall {
  pub(crate) arguments: String,
  pub(crate) id: String,
  pub(crate) name: String,
}

impl ToolCall {
  pub(crate) fn invocation(&self) -> Result<ToolInvocation> {
    let arguments = serde_json::from_str::<Value>(&self.arguments)
      .with_context(|| format!("failed to parse `{}` arguments", self.name))?;

    ToolInvocation::from_json(&self.name, &arguments)
  }

  pub(crate) fn new(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl Into<String>,
  ) -> Self {
    Self {
      arguments: arguments.into(),
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolDefinition {
  description: &'static str,
  name: &'static str,
  parameters: Value,
}

impl ToolDefinition {
  pub(crate) fn anthropic(&self) -> types::Tool {
    let Value::Object(schema) = self.parameters.clone() else {
      unreachable!()
    };

    let properties = schema
      .get("properties")
      .and_then(Value::as_object)
      .cloned()
      .unwrap_or_default();

    let required = schema
      .get("required")
      .and_then(Value::as_array)
      .into_iter()
      .flatten()
      .filter_map(Value::as_str)
      .map(str::to_string)
      .collect();

    let additional = schema
      .into_iter()
      .filter(|(key, _)| {
        key != "properties" && key != "required" && key != "type"
      })
      .collect();

    types::Tool {
      description: self.description.into(),
      input_schema: types::ToolInputSchema {
        additional,
        properties,
        required,
        schema_type: "object".into(),
      },
      name: self.name.into(),
    }
  }

  fn new(
    name: &'static str,
    description: &'static str,
    parameters: Value,
  ) -> Self {
    Self {
      description,
      name,
      parameters,
    }
  }

  pub(crate) fn openai(&self) -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
      function: FunctionObject {
        description: Some(self.description.into()),
        name: self.name.into(),
        parameters: Some(self.parameters.clone()),
        strict: None,
      },
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocation {
  ApplyPatch {
    cwd: Option<PathBuf>,
    patch: String,
  },
  Command {
    arguments: Vec<String>,
    cwd: Option<PathBuf>,
    program: String,
  },
  ListFiles {
    cwd: Option<PathBuf>,
  },
  ReadFile {
    path: PathBuf,
  },
  Rg {
    arguments: Vec<String>,
    cwd: Option<PathBuf>,
  },
}

impl ToolInvocation {
  fn action(&self, tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Past => match self {
        Self::ListFiles { .. } => "Listed",
        Self::ReadFile { .. } => "Read",
        Self::ApplyPatch { .. } | Self::Command { .. } | Self::Rg { .. } => {
          "Ran"
        }
      },
      ToolActionTense::Progressive => match self {
        Self::ListFiles { .. } => "Listing",
        Self::ReadFile { .. } => "Reading",
        Self::ApplyPatch { .. } | Self::Command { .. } | Self::Rg { .. } => {
          "Running"
        }
      },
    }
  }

  fn command_line(program: &str, arguments: &[String]) -> String {
    if arguments.is_empty() {
      program.into()
    } else {
      format!("{program} {}", arguments.join(" "))
    }
  }

  fn field<'a>(arguments: &'a Value, name: &str) -> Result<&'a Value> {
    arguments
      .get(name)
      .with_context(|| format!("missing `{name}`"))
  }

  fn from_json(name: &str, arguments: &Value) -> Result<Self> {
    match name {
      "apply_patch" => Ok(Self::ApplyPatch {
        cwd: Self::optional_path(arguments, "cwd")?,
        patch: Self::required_string(arguments, "patch")?,
      }),
      "command" => Ok(Self::Command {
        arguments: Self::string_array(arguments, "arguments")?,
        cwd: Self::optional_path(arguments, "cwd")?,
        program: Self::required_string(arguments, "program")?,
      }),
      "list_files" => Ok(Self::ListFiles {
        cwd: Self::optional_path(arguments, "cwd")?,
      }),
      "read_file" => Ok(Self::ReadFile {
        path: PathBuf::from(Self::required_string(arguments, "path")?),
      }),
      "rg" => Ok(Self::Rg {
        arguments: Self::string_array(arguments, "arguments")?,
        cwd: Self::optional_path(arguments, "cwd")?,
      }),
      _ => bail!("unknown tool `{name}`"),
    }
  }

  fn optional_path(arguments: &Value, name: &str) -> Result<Option<PathBuf>> {
    let Some(value) = arguments.get(name) else {
      return Ok(None);
    };

    if value.is_null() {
      return Ok(None);
    }

    Ok(Some(PathBuf::from(
      value
        .as_str()
        .with_context(|| format!("`{name}` must be a string"))?,
    )))
  }

  pub(crate) fn past_tense(&self) -> String {
    self.title(ToolActionTense::Past)
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn required_string(arguments: &Value, name: &str) -> Result<String> {
    Ok(
      Self::field(arguments, name)?
        .as_str()
        .with_context(|| format!("`{name}` must be a string"))?
        .into(),
    )
  }

  fn string_array(arguments: &Value, name: &str) -> Result<Vec<String>> {
    Self::field(arguments, name)?
      .as_array()
      .with_context(|| format!("`{name}` must be an array"))?
      .iter()
      .map(|value| {
        Ok(
          value
            .as_str()
            .with_context(|| format!("`{name}` entries must be strings"))?
            .into(),
        )
      })
      .collect()
  }

  fn subject(&self) -> String {
    match self {
      Self::ApplyPatch { .. } => "apply_patch".into(),
      Self::Command {
        arguments, program, ..
      } => Self::command_line(program, arguments),
      Self::ListFiles { cwd } => cwd.as_ref().map_or_else(
        || "files".into(),
        |cwd| format!("files in {}", cwd.display()),
      ),
      Self::ReadFile { path } => path.display().to_string(),
      Self::Rg { arguments, .. } => Self::command_line("rg", arguments),
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
    match self {
      Self::ApplyPatch { .. } => write!(f, "apply_patch"),
      Self::Command {
        arguments, program, ..
      } => write!(f, "{}", Self::command_line(program, arguments)),
      Self::ListFiles { cwd } => {
        if let Some(cwd) = cwd {
          write!(f, "list files in {}", cwd.display())
        } else {
          write!(f, "list files")
        }
      }
      Self::ReadFile { path } => write!(f, "read {}", path.display()),
      Self::Rg { arguments, .. } => {
        write!(f, "{}", Self::command_line("rg", arguments))
      }
    }
  }
}

pub(crate) fn tool_definitions() -> Vec<ToolDefinition> {
  vec![
    ToolDefinition::new(
      "list_files",
      "List project files while respecting .gitignore and other standard ignore rules.",
      json!({
        "type": "object",
        "properties": {
          "cwd": {"type": ["string", "null"]}
        },
        "required": [],
        "additionalProperties": false
      }),
    ),
    ToolDefinition::new(
      "rg",
      "Search files with ripgrep.",
      json!({
        "type": "object",
        "properties": {
          "arguments": {
            "type": "array",
            "items": {"type": "string"}
          },
          "cwd": {"type": ["string", "null"]}
        },
        "required": ["arguments"],
        "additionalProperties": false
      }),
    ),
    ToolDefinition::new(
      "read_file",
      "Read a UTF-8 text file.",
      json!({
        "type": "object",
        "properties": {
          "path": {"type": "string"}
        },
        "required": ["path"],
        "additionalProperties": false
      }),
    ),
    ToolDefinition::new(
      "command",
      "Run a command and capture stdout, stderr, and exit status. Do not use this to list project files; use list_files instead.",
      json!({
        "type": "object",
        "properties": {
          "program": {"type": "string"},
          "arguments": {
            "type": "array",
            "items": {"type": "string"}
          },
          "cwd": {"type": ["string", "null"]}
        },
        "required": ["program", "arguments"],
        "additionalProperties": false
      }),
    ),
    ToolDefinition::new(
      "apply_patch",
      "Apply a unified patch to the workspace.",
      json!({
        "type": "object",
        "properties": {
          "patch": {"type": "string"},
          "cwd": {"type": ["string", "null"]}
        },
        "required": ["patch"],
        "additionalProperties": false
      }),
    ),
  ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolOutput {
  ApplyPatch(CommandOutput),
  Command(CommandOutput),
  ListFiles(CommandOutput),
  ReadFile(String),
  Rg(CommandOutput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolResult {
  pub(crate) invocation: ToolInvocation,
  pub(crate) output: std::result::Result<ToolOutput, ToolError>,
}

impl ToolResult {
  pub(crate) fn error(
    invocation: ToolInvocation,
    message: impl Into<String>,
  ) -> Self {
    Self {
      invocation,
      output: Err(ToolError::new(message)),
    }
  }

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
      Self::ApplyPatch(output)
      | Self::Command(output)
      | Self::ListFiles(output)
      | Self::Rg(output) => Self::command_content(output),
      Self::ReadFile(content) => content.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_tool_calls() {
    #[track_caller]
    fn case(call: &ToolCall, expected: &ToolInvocation) {
      assert_eq!(&call.invocation().unwrap(), expected);
    }

    case(
      &ToolCall::new("foo", "rg", r#"{"arguments":["foo"],"cwd":"bar"}"#),
      &ToolInvocation::Rg {
        arguments: vec!["foo".into()],
        cwd: Some("bar".into()),
      },
    );

    case(
      &ToolCall::new("foo", "read_file", r#"{"path":"bar"}"#),
      &ToolInvocation::ReadFile { path: "bar".into() },
    );

    case(
      &ToolCall::new("foo", "list_files", r#"{"cwd":"bar"}"#),
      &ToolInvocation::ListFiles {
        cwd: Some("bar".into()),
      },
    );

    case(
      &ToolCall::new(
        "foo",
        "command",
        r#"{"program":"bar","arguments":["baz"],"cwd":null}"#,
      ),
      &ToolInvocation::Command {
        arguments: vec!["baz".into()],
        cwd: None,
        program: "bar".into(),
      },
    );

    case(
      &ToolCall::new("foo", "apply_patch", r#"{"patch":"bar"}"#),
      &ToolInvocation::ApplyPatch {
        cwd: None,
        patch: "bar".into(),
      },
    );
  }

  #[test]
  fn unknown_tool_errors() {
    assert_eq!(
      ToolCall::new("foo", "bar", "{}")
        .invocation()
        .unwrap_err()
        .to_string(),
      "unknown tool `bar`",
    );
  }
}
