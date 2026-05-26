use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInvocation {
  pub(crate) id: String,
  pub(crate) kind: ToolInvocationKind,
}

impl ToolInvocation {
  fn action(&self, tense: ToolActionTense) -> &'static str {
    match &self.kind {
      ToolInvocationKind::ApplyPatchTool(_) => match tense {
        ToolActionTense::Completed => "Applied",
        ToolActionTense::Failed => "Failed applying",
        ToolActionTense::Progressive => "Applying",
      },
      ToolInvocationKind::CommandTool(_) => match tense {
        ToolActionTense::Completed => "Ran",
        ToolActionTense::Failed => "Failed running",
        ToolActionTense::Progressive => "Running",
      },
      ToolInvocationKind::ListFilesTool(_) => match tense {
        ToolActionTense::Completed => "Listed",
        ToolActionTense::Failed => "Failed listing",
        ToolActionTense::Progressive => "Listing",
      },
      ToolInvocationKind::ReadFileTool(_) => match tense {
        ToolActionTense::Completed => "Read",
        ToolActionTense::Failed => "Failed reading",
        ToolActionTense::Progressive => "Reading",
      },
      ToolInvocationKind::SearchFilesTool(_) => match tense {
        ToolActionTense::Completed => "Searched",
        ToolActionTense::Failed => "Failed searching",
        ToolActionTense::Progressive => "Searching",
      },
    }
  }

  fn arguments(&self) -> Value {
    match &self.kind {
      ToolInvocationKind::ApplyPatchTool(tool) => {
        serde_json::to_value(tool).expect("failed to serialize tool arguments")
      }
      ToolInvocationKind::CommandTool(tool) => {
        serde_json::to_value(tool).expect("failed to serialize tool arguments")
      }
      ToolInvocationKind::ListFilesTool(tool) => {
        serde_json::to_value(tool).expect("failed to serialize tool arguments")
      }
      ToolInvocationKind::ReadFileTool(tool) => {
        serde_json::to_value(tool).expect("failed to serialize tool arguments")
      }
      ToolInvocationKind::SearchFilesTool(tool) => {
        serde_json::to_value(tool).expect("failed to serialize tool arguments")
      }
    }
  }

  fn command(&self) -> Option<&CommandTool> {
    match &self.kind {
      ToolInvocationKind::CommandTool(command) => Some(command),
      ToolInvocationKind::ApplyPatchTool(_)
      | ToolInvocationKind::ListFilesTool(_)
      | ToolInvocationKind::ReadFileTool(_)
      | ToolInvocationKind::SearchFilesTool(_) => None,
    }
  }

  pub(crate) fn completed_tense(&self) -> String {
    self.title(ToolActionTense::Completed)
  }

  pub(crate) fn failed_tense(&self) -> String {
    self.title(ToolActionTense::Failed)
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

  pub(crate) fn message(&self) -> Message {
    Message::tool_use(self.id.clone(), self.name(), self.arguments())
  }

  fn name(&self) -> &'static str {
    match &self.kind {
      ToolInvocationKind::ApplyPatchTool(_) => "apply_patch",
      ToolInvocationKind::CommandTool(_) => "command",
      ToolInvocationKind::ListFilesTool(_) => "list_files",
      ToolInvocationKind::ReadFileTool(_) => "read_file",
      ToolInvocationKind::SearchFilesTool(_) => "search_files",
    }
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn subject(&self) -> String {
    match &self.kind {
      ToolInvocationKind::ApplyPatchTool(_) => "apply_patch".into(),
      ToolInvocationKind::CommandTool(_) => self
        .command()
        .map_or_else(|| "command".into(), ToString::to_string),
      ToolInvocationKind::ListFilesTool(tool) => tool.cwd.as_ref().map_or_else(
        || "files".into(),
        |cwd| format!("files in {}", cwd.display()),
      ),
      ToolInvocationKind::ReadFileTool(tool) => tool.path.display().to_string(),
      ToolInvocationKind::SearchFilesTool(tool) => {
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
      ToolInvocationKind::ApplyPatchTool(_) => write!(f, "apply_patch"),
      ToolInvocationKind::CommandTool(_) => {
        if let Some(command) = self.command() {
          write!(f, "{command}")
        } else {
          write!(f, "command")
        }
      }
      ToolInvocationKind::ListFilesTool(tool) => {
        if let Some(cwd) = &tool.cwd {
          write!(f, "list files in {}", cwd.display())
        } else {
          write!(f, "list files")
        }
      }
      ToolInvocationKind::ReadFileTool(tool) => {
        write!(f, "read {}", tool.path.display())
      }
      ToolInvocationKind::SearchFilesTool(tool) => {
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
        kind: ToolInvocationKind::ApplyPatchTool(ApplyPatchTool {
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
        kind: ToolInvocationKind::CommandTool(CommandTool {
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
        kind: ToolInvocationKind::ListFilesTool(ListFilesTool {
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
        kind: ToolInvocationKind::ReadFileTool(ReadFileTool {
          path: "bar".into()
        }),
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
        kind: ToolInvocationKind::SearchFilesTool(SearchFilesTool {
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
