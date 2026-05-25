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
    }
  ) => {
    #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct $type {
      $(pub(crate) $field: $field_type,)*
    }

    impl Tool for $type {
      const DESCRIPTION: &'static str = $description;

      const NAME: &'static str = $name;

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

    impl From<$type> for ToolInvocationKind {
      fn from(tool: $type) -> Self {
        Self::$type(tool)
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

pub(crate) mod apply_patch;
pub(crate) mod command;
pub(crate) mod list_files;
pub(crate) mod read_file;
pub(crate) mod search_files;

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

pub(crate) trait Tool:
  Into<ToolInvocationKind> + DeserializeOwned + Sized
{
  const DESCRIPTION: &'static str;

  const NAME: &'static str;

  fn decode(name: &str, arguments: Value) -> Result<Self> {
    serde_json::from_value(arguments)
      .with_context(|| format!("failed to decode `{name}` arguments"))
  }

  fn parameters() -> Value;

  fn parse(call: RawToolCall) -> Result<ToolInvocation> {
    let RawToolCall {
      arguments,
      id,
      name,
    } = call;

    Ok(ToolInvocation {
      id,
      kind: Self::decode(&name, arguments)?.into(),
    })
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
        kind: ToolInvocationKind::ApplyPatch(apply_patch::ApplyPatch {
          cwd: None,
          patch: "bar".into(),
        }),
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
        kind: ToolInvocationKind::Command(command::Command {
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
        kind: ToolInvocationKind::ListFiles(list_files::ListFiles {
          cwd: Some("bar".into()),
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
        kind: ToolInvocationKind::ReadFile(read_file::ReadFile {
          path: "bar".into(),
        }),
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
        kind: ToolInvocationKind::SearchFiles(search_files::SearchFiles {
          arguments: vec!["foo".into()],
          cwd: Some("bar".into()),
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
