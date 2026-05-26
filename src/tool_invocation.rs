use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInvocation {
  pub(crate) id: String,
  pub(crate) kind: ToolInvocationKind,
}

impl ToolInvocation {
  fn action(&self, tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Progressive => match &self.kind {
        ToolInvocationKind::ListFiles(_) => "Listing",
        ToolInvocationKind::ReadFile(_) => "Reading",
        ToolInvocationKind::SearchFiles(_) => "Searching",
        _ => "Running",
      },
    }
  }

  fn command(&self) -> Option<&CommandTool> {
    match &self.kind {
      ToolInvocationKind::Command(command) => Some(command),
      ToolInvocationKind::ApplyPatch(_)
      | ToolInvocationKind::ListFiles(_)
      | ToolInvocationKind::ReadFile(_)
      | ToolInvocationKind::SearchFiles(_) => None,
    }
  }

  pub(crate) fn from_raw<T>(call: RawToolCall) -> Result<ToolInvocationKind>
  where
    T: Into<ToolInvocationKind> + DeserializeOwned,
  {
    Ok(
      serde_json::from_value::<T>(call.arguments)
        .with_context(|| format!("failed to decode `{}` arguments", call.name))?
        .into(),
    )
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn subject(&self) -> String {
    match &self.kind {
      ToolInvocationKind::ApplyPatch(_) => "apply_patch".into(),
      ToolInvocationKind::Command(_) => self
        .command()
        .map_or_else(|| "command".into(), ToString::to_string),
      ToolInvocationKind::ListFiles(tool) => tool.cwd.as_ref().map_or_else(
        || "files".into(),
        |cwd| format!("files in {}", cwd.display()),
      ),
      ToolInvocationKind::ReadFile(tool) => tool.path.display().to_string(),
      ToolInvocationKind::SearchFiles(tool) => {
        let query = if tool.arguments.is_empty() {
          "files".into()
        } else {
          tool.arguments.join(" ")
        };

        tool
          .cwd
          .as_ref()
          .map_or(query.clone(), |cwd| format!("{query} in {}", cwd.display()))
      }
    }
  }

  fn title(&self, tense: ToolActionTense) -> String {
    format!("{} {}", self.action(tense), self.subject())
  }
}

impl Display for ToolInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match &self.kind {
      ToolInvocationKind::ApplyPatch(_) => write!(f, "apply_patch"),
      ToolInvocationKind::Command(_) => {
        if let Some(command) = self.command() {
          write!(f, "{command}")
        } else {
          write!(f, "command")
        }
      }
      ToolInvocationKind::ListFiles(tool) => {
        if let Some(cwd) = &tool.cwd {
          write!(f, "list files in {}", cwd.display())
        } else {
          write!(f, "list files")
        }
      }
      ToolInvocationKind::ReadFile(tool) => {
        write!(f, "read {}", tool.path.display())
      }
      ToolInvocationKind::SearchFiles(tool) => {
        if tool.arguments.is_empty() {
          if let Some(cwd) = &tool.cwd {
            write!(f, "search files in {}", cwd.display())
          } else {
            write!(f, "search files")
          }
        } else if let Some(cwd) = &tool.cwd {
          write!(
            f,
            "search files {} in {}",
            tool.arguments.join(" "),
            cwd.display()
          )
        } else {
          write!(f, "search files {}", tool.arguments.join(" "))
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_apply_patch_tool_call() {
    let invocation: ToolInvocation =
      RawToolCall::new("foo", "apply_patch", json!({"patch": "bar"}))
        .try_into()
        .unwrap();

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
    let invocation: ToolInvocation = RawToolCall::new(
      "foo",
      "command",
      json!({"program": "bar", "arguments": ["baz"], "cwd": null}),
    )
    .try_into()
    .unwrap();

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
    let invocation: ToolInvocation =
      RawToolCall::new("foo", "list_files", json!({"cwd": "bar"}))
        .try_into()
        .unwrap();

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
    let invocation: ToolInvocation =
      RawToolCall::new("foo", "read_file", json!({"path": "bar"}))
        .try_into()
        .unwrap();

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::ReadFile(ReadFileTool { path: "bar".into() }),
      },
    );
  }

  #[test]
  fn parses_search_files_tool_call() {
    let invocation: ToolInvocation = RawToolCall::new(
      "foo",
      "search_files",
      json!({"arguments": ["foo"], "cwd": "bar"}),
    )
    .try_into()
    .unwrap();

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
  fn unknown_tool_errors() {
    let result: Result<ToolInvocation> =
      RawToolCall::new("foo", "bar", json!({})).try_into();

    assert_eq!(result.unwrap_err().to_string(), "unknown tool `bar`",);
  }
}
