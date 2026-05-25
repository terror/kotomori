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

inventory::collect!(RegisteredTool);

mod apply_patch;
mod command;
mod list_files;
mod read_file;
mod search_files;

pub(crate) enum ToolCallFragment<I> {
  Finish {
    index: I,
  },
  Update {
    argument_fragment: Option<String>,
    arguments: Option<Value>,
    id: Option<String>,
    index: I,
    name: Option<String>,
  },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawToolCall {
  pub(crate) arguments: Value,
  pub(crate) id: String,
  pub(crate) name: String,
}

pub(crate) trait ToolCallStreamEvent {
  type Index: Ord;

  fn tool_call_fragment(self) -> Option<ToolCallFragment<Self::Index>>;
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

impl ToolCallStreamEvent
  for openai::types::chat::ChatCompletionMessageToolCallChunk
{
  type Index = u32;

  fn tool_call_fragment(self) -> Option<ToolCallFragment<Self::Index>> {
    let (name, argument_fragment) = self
      .function
      .map_or((None, None), |function| (function.name, function.arguments));

    Some(ToolCallFragment::Update {
      argument_fragment,
      arguments: None,
      id: self.id,
      index: self.index,
      name,
    })
  }
}

impl ToolCallStreamEvent for anthropic::types::MessageStreamEvent {
  type Index = usize;

  fn tool_call_fragment(self) -> Option<ToolCallFragment<Self::Index>> {
    match self {
      Self::ContentBlockStart {
        content_block:
          anthropic::types::ContentBlock::ToolUse { id, input, name },
        index,
      } => Some(ToolCallFragment::Update {
        argument_fragment: None,
        arguments: Some(input),
        id: Some(id),
        index,
        name: Some(name),
      }),
      Self::ContentBlockDelta {
        delta:
          anthropic::types::ContentBlockDelta::InputJsonDelta { partial_json },
        index,
      } => Some(ToolCallFragment::Update {
        argument_fragment: Some(partial_json),
        arguments: None,
        id: None,
        index,
        name: None,
      }),
      Self::ContentBlockStop { index } => {
        Some(ToolCallFragment::Finish { index })
      }
      _ => None,
    }
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

impl ToolInvocation {
  fn action(&self, tense: ToolActionTense) -> &'static str {
    match tense {
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

impl From<&RegisteredTool> for openai::types::chat::ChatCompletionTools {
  fn from(tool: &RegisteredTool) -> Self {
    Self::Function(openai::types::chat::ChatCompletionTool {
      function: openai::types::chat::FunctionObject {
        description: Some(tool.description.into()),
        name: tool.name.into(),
        parameters: Some(tool.parameters()),
        strict: None,
      },
    })
  }
}

impl From<&RegisteredTool> for anthropic::types::Tool {
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
      input_schema: anthropic::types::ToolInputSchema {
        additional,
        properties,
        required,
        schema_type: "object".into(),
      },
      name: tool.name.into(),
    }
  }
}

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
