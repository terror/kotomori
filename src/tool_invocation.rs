use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInvocation {
  pub(crate) id: String,
  pub(crate) kind: ToolInvocationKind,
}

impl ToolInvocation {
  pub(crate) fn arguments(&self) -> Value {
    self.kind.arguments()
  }

  pub(crate) fn completed_tense(&self) -> String {
    self.title(ToolActionTense::Completed)
  }

  pub(crate) fn failed_tense(&self) -> String {
    self.title(ToolActionTense::Failed)
  }

  pub(crate) fn from_raw<T>(call: RawToolCall) -> Result<ToolInvocationKind>
  where
    T: ToolSpec,
  {
    Ok(
      serde_json::from_value::<T>(call.arguments)
        .with_context(|| format!("failed to decode `{}` arguments", call.name))?
        .into(),
    )
  }

  pub(crate) fn message(&self) -> Message {
    Message::Agent(vec![AgentMessageContent::ToolCall(self.clone())])
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn title(&self, tense: ToolActionTense) -> String {
    format!("{} {}", self.kind.action(tense), self.kind.subject())
  }
}

impl Display for ToolInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", self.kind.display())
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  fn invocation(name: &str, arguments: Value) -> ToolInvocation {
    ToolRegistry::default()
      .invocation(RawToolCall::new("foo", name, arguments))
      .unwrap()
  }

  #[test]
  fn parses_apply_patch_tool_call() {
    let invocation = invocation("apply_patch", json!({"patch": "bar"}));

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ApplyPatch(ApplyPatchTool {
          cwd: None,
          patch: "bar".into(),
        }),
      },
    );
  }

  #[test]
  fn parses_command_tool_call() {
    let invocation = invocation(
      "command",
      json!({"program": "bar", "arguments": ["baz"], "cwd": null}),
    );

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::Command(CommandTool {
          arguments: vec!["baz".into()],
          cwd: None,
          program: "bar".into(),
        }),
      },
    );
  }

  #[test]
  fn parses_list_files_tool_call() {
    let invocation = invocation("list_files", json!({"cwd": "bar"}));

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ListFiles(ListFilesTool {
          cwd: Some("bar".into()),
        }),
      },
    );
  }

  #[test]
  fn parses_read_file_tool_call() {
    let invocation = invocation("read_file", json!({"path": "bar"}));

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ReadFile(ReadFileTool {
          cwd: None,
          end_line: None,
          path: "bar".into(),
          start_line: None,
        }),
      },
    );
  }

  #[test]
  fn parses_search_files_tool_call() {
    let invocation =
      invocation("search_files", json!({"arguments": ["foo"], "cwd": "bar"}));

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::SearchFiles(SearchFilesTool {
          arguments: vec!["foo".into()],
          cwd: Some("bar".into()),
        }),
      },
    );
  }

  #[test]
  fn serializes_tool_arguments() {
    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "baz".into(),
      }),
    };

    assert_eq!(
      invocation.arguments(),
      json!({"arguments": ["bar"], "program": "baz"}),
    );
  }

  #[test]
  fn parses_write_file_tool_call() {
    let invocation =
      invocation("write_file", json!({"content": "bar", "path": "baz"}));

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::WriteFile(WriteFileTool {
          content: "bar".into(),
          cwd: None,
          path: "baz".into(),
        }),
      },
    );
  }

  #[test]
  fn unknown_tool_errors() {
    let result = ToolRegistry::default().invocation(RawToolCall::new(
      "foo",
      "bar",
      json!({}),
    ));

    assert_eq!(result.unwrap_err().to_string(), "unknown tool `bar`",);
  }
}
