use super::*;

macro_rules! define_tool {
  (
    $type:ident {
      name: $name:literal,
      description: $description:literal,
      arguments {
        $(
          $presence:ident $field:ident: $field_type:ty => $schema:tt
        ),* $(,)?
      }
      invocation |$tool:ident| $invocation:expr $(,)?
    }
  ) => {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct $type {
      $($field: $field_type,)*
    }

    impl Tool for $type {
      const DESCRIPTION: &'static str = $description;

      const NAME: &'static str = $name;

      fn invocation(self, id: String) -> ToolInvocation {
        let $tool = self;

        ToolInvocation {
          id,
          kind: $invocation,
        }
      }

      fn parameters() -> Value {
        let required = [
          $(define_tool!(@required $presence $field),)*
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        json!({
          "type": "object",
          "properties": {
            $(stringify!($field): $schema,)*
          },
          "required": required,
          "additionalProperties": false
        })
      }
    }

    inventory::submit! {
      RegisteredTool {
        description: <$type as Tool>::DESCRIPTION,
        invocation: <$type as Tool>::parse,
        name: <$type as Tool>::NAME,
        parameters: <$type as Tool>::parameters,
      }
    }
  };
  (@required required $field:ident) => {
    Some(stringify!($field))
  };
  (@required optional $field:ident) => {
    None::<&str>
  };
}

mod apply_patch;
mod command;
mod list_files;
mod read_file;
mod search_files;

#[derive(Default)]
pub(crate) struct PendingToolCall {
  pub(crate) arguments: String,
  pub(crate) id: Option<String>,
  pub(crate) name: Option<String>,
}

impl PendingToolCall {
  pub(crate) fn append(&mut self, chunk: ChatCompletionMessageToolCallChunk) {
    if let Some(id) = chunk.id {
      self.id = Some(id);
    }

    if let Some(function) = chunk.function {
      if let Some(name) = function.name {
        self.name = Some(name);
      }

      self.append_arguments(function.arguments);
    }
  }

  pub(crate) fn append_arguments(&mut self, arguments: Option<String>) {
    if let Some(arguments) = arguments {
      self.arguments.push_str(&arguments);
    }
  }

  fn arguments(arguments: Value) -> String {
    match arguments {
      Value::Null => String::new(),
      Value::Object(object) if object.is_empty() => String::new(),
      arguments => arguments.to_string(),
    }
  }

  pub(crate) fn finish(self) -> Result<RawToolCall> {
    let id = self.id.context("missing tool call id")?;

    let name = self.name.context("missing tool call name")?;

    let arguments = if self.arguments.trim().is_empty() {
      "{}"
    } else {
      &self.arguments
    };

    RawToolCall::from_arguments_string(id, name, arguments)
  }

  pub(crate) fn new(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: Value,
  ) -> Self {
    Self {
      arguments: Self::arguments(arguments),
      id: Some(id.into()),
      name: Some(name.into()),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawToolCall {
  pub(crate) arguments: Value,
  pub(crate) id: String,
  pub(crate) name: String,
}

impl RawToolCall {
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
    inventory::iter::<RegisteredTool>
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

impl Display for RawToolCall {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{} {}", self.name, self.arguments)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInvocation {
  pub(crate) id: String,
  pub(crate) kind: ToolInvocationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch { cwd: Option<PathBuf>, patch: String },
  Command(CommandInvocation),
  ListFiles(CommandInvocation),
  ReadFile { path: PathBuf },
  SearchFiles(CommandInvocation),
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

impl ToolInvocation {
  fn action(&self, tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Past => match &self.kind {
        ToolInvocationKind::ListFiles(_) => "Listed",
        ToolInvocationKind::ReadFile { .. } => "Read",
        _ => "Ran",
      },
      ToolActionTense::Progressive => match &self.kind {
        ToolInvocationKind::ListFiles(_) => "Listing",
        ToolInvocationKind::ReadFile { .. } => "Reading",
        _ => "Running",
      },
    }
  }

  fn command(&self) -> Option<&CommandInvocation> {
    match &self.kind {
      ToolInvocationKind::Command(command)
      | ToolInvocationKind::ListFiles(command)
      | ToolInvocationKind::SearchFiles(command) => Some(command),
      ToolInvocationKind::ApplyPatch { .. }
      | ToolInvocationKind::ReadFile { .. } => None,
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
    match &self.kind {
      ToolInvocationKind::ApplyPatch { .. } => "apply_patch".into(),
      ToolInvocationKind::Command(_) | ToolInvocationKind::SearchFiles(_) => {
        self
          .command()
          .map_or_else(|| "command".into(), ToString::to_string)
      }
      ToolInvocationKind::ListFiles(_) => self
        .command()
        .and_then(|command| command.cwd.as_ref())
        .map_or_else(
          || "files".into(),
          |cwd| format!("files in {}", cwd.display()),
        ),
      ToolInvocationKind::ReadFile { path } => path.display().to_string(),
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
    match &self.kind {
      ToolInvocationKind::ApplyPatch { .. } => write!(f, "apply_patch"),
      ToolInvocationKind::Command(_) | ToolInvocationKind::SearchFiles(_) => {
        if let Some(command) = self.command() {
          write!(f, "{command}")
        } else {
          write!(f, "command")
        }
      }
      ToolInvocationKind::ListFiles(_) => {
        if let Some(cwd) =
          self.command().and_then(|command| command.cwd.as_ref())
        {
          write!(f, "list files in {}", cwd.display())
        } else {
          write!(f, "list files")
        }
      }
      ToolInvocationKind::ReadFile { path } => {
        write!(f, "read {}", path.display())
      }
    }
  }
}

#[derive(Clone, Copy)]
pub(crate) struct RegisteredTool {
  pub(crate) description: &'static str,
  invocation: fn(RawToolCall) -> Result<ToolInvocation>,
  pub(crate) name: &'static str,
  parameters: fn() -> Value,
}

impl RegisteredTool {
  pub(crate) fn invocation(&self, call: RawToolCall) -> Result<ToolInvocation> {
    (self.invocation)(call)
  }

  pub(crate) fn parameters(&self) -> Value {
    (self.parameters)()
  }
}

impl From<&RegisteredTool> for ChatCompletionTools {
  fn from(tool: &RegisteredTool) -> Self {
    Self::Function(ChatCompletionTool {
      function: FunctionObject {
        description: Some(tool.description.into()),
        name: tool.name.into(),
        parameters: Some(tool.parameters()),
        strict: None,
      },
    })
  }
}

impl From<&RegisteredTool> for types::Tool {
  fn from(tool: &RegisteredTool) -> Self {
    let Value::Object(schema) = tool.parameters() else {
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

    Self {
      description: tool.description.into(),
      input_schema: types::ToolInputSchema {
        additional,
        properties,
        required,
        schema_type: "object".into(),
      },
      name: tool.name.into(),
    }
  }
}

inventory::collect!(RegisteredTool);

pub(crate) trait Tool: serde::de::DeserializeOwned + Sized {
  const DESCRIPTION: &'static str;

  const NAME: &'static str;

  fn decode(call: &RawToolCall) -> Result<Self> {
    serde_json::from_value(call.arguments.clone())
      .with_context(|| format!("failed to decode `{}` arguments", call.name))
  }

  fn invocation(self, id: String) -> ToolInvocation;

  fn parameters() -> Value;

  fn parse(call: RawToolCall) -> Result<ToolInvocation> {
    Ok(Self::decode(&call)?.invocation(call.id))
  }
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
      RawToolCall::new("foo", "apply_patch", json!({"patch": "bar"}))
        .invocation()
        .unwrap(),
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ApplyPatch {
          cwd: None,
          patch: "bar".into(),
        },
      },
    );
  }

  #[test]
  fn parses_command_tool_call() {
    assert_eq!(
      RawToolCall::new(
        "foo",
        "command",
        json!({"program": "bar", "arguments": ["baz"], "cwd": null}),
      )
      .invocation()
      .unwrap(),
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::Command(CommandInvocation {
          arguments: vec!["baz".into()],
          cwd: None,
          program: "bar".into(),
        }),
      },
    );
  }

  #[test]
  fn parses_list_files_tool_call() {
    assert_eq!(
      RawToolCall::new("foo", "list_files", json!({"cwd": "bar"}))
        .invocation()
        .unwrap(),
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ListFiles(CommandInvocation {
          arguments: vec!["--files".into()],
          cwd: Some("bar".into()),
          program: "rg".into(),
        }),
      },
    );
  }

  #[test]
  fn parses_read_file_tool_call() {
    assert_eq!(
      RawToolCall::new("foo", "read_file", json!({"path": "bar"}))
        .invocation()
        .unwrap(),
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ReadFile { path: "bar".into() },
      },
    );
  }

  #[test]
  fn parses_search_files_tool_call() {
    assert_eq!(
      RawToolCall::new(
        "foo",
        "search_files",
        json!({"arguments": ["foo"], "cwd": "bar"}),
      )
      .invocation()
      .unwrap(),
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::SearchFiles(CommandInvocation {
          arguments: vec!["foo".into()],
          cwd: Some("bar".into()),
          program: "rg".into(),
        }),
      },
    );
  }

  #[test]
  fn parses_tool_call_arguments() {
    assert_eq!(
      RawToolCall::from_arguments_string(
        "foo",
        "read_file",
        r#"{"path":"bar"}"#
      )
      .unwrap(),
      RawToolCall::new("foo", "read_file", json!({"path": "bar"})),
    );
  }

  #[test]
  fn unknown_tool_errors() {
    assert_eq!(
      RawToolCall::new("foo", "bar", json!({}))
        .invocation()
        .unwrap_err()
        .to_string(),
      "unknown tool `bar`",
    );
  }
}
