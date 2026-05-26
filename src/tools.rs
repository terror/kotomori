use super::*;

macro_rules! define_tools {
  (
    $(
      $tool:ident {
        name: $name:literal,
        description: $description:literal,
        fields: {
          $(
            $field:ident: $field_ty:ty
          ),* $(,)?
        },
      }
    ),* $(,)?
  ) => {
    $(
      #[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
      #[serde(deny_unknown_fields)]
      pub(crate) struct $tool {
        $(
          pub(crate) $field: $field_ty,
        )*
      }

      impl From<$tool> for ToolInvocationKind {
        fn from(tool: $tool) -> Self {
          Self::$tool(tool)
        }
      }
    )*

    pub(crate) static TOOLS: LazyLock<Vec<Tool>> = LazyLock::new(|| {
      vec![
        $(
          Tool {
            name: $name,
            description: $description,
            invocation: ToolInvocation::from_raw::<$tool>,
            parameters: serde_json::to_value(
              <$tool as schemars::JsonSchema>::json_schema(
                &mut schemars::SchemaGenerator::default(),
              ),
            )
            .expect("failed to serialize tool schema"),
          },
        )*
      ]
    });
  };
}

define_tools! {
  ApplyPatchTool {
    name: "apply_patch",
    description: "Apply a unified patch to the workspace.",
    fields: {
      cwd: Option<PathBuf>,
      patch: String,
    },
  },
  CommandTool {
    name: "command",
    description: "Run a command and capture stdout, stderr, and exit status. Do not use this to list project files; use list_files instead.",
    fields: {
      arguments: Vec<String>,
      cwd: Option<PathBuf>,
      program: String,
    },
  },
  ListFilesTool {
    name: "list_files",
    description: "List project files while respecting .gitignore and other standard ignore rules.",
    fields: {
      cwd: Option<PathBuf>,
    },
  },
  ReadFileTool {
    name: "read_file",
    description: "Read a UTF-8 text file.",
    fields: {
      path: PathBuf,
    },
  },
  SearchFilesTool {
    name: "search_files",
    description: "Search files with ripgrep.",
    fields: {
      arguments: Vec<String>,
      cwd: Option<PathBuf>,
    },
  },
}

impl ApplyPatchTool {
  pub(crate) fn execute(&self) -> ToolResult {
    let mut command = process::Command::new("apply_patch");

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    let mut child = match command
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
    {
      Ok(child) => child,
      Err(error) => return ToolResult::error(&error),
    };

    let Some(mut stdin) = child.stdin.take() else {
      return ToolResult::error(&"failed to open apply_patch stdin");
    };

    if let Err(error) = stdin.write_all(self.patch.as_bytes()) {
      return ToolResult::error(&error);
    }

    drop(stdin);

    ToolResult::output(child.wait_with_output())
  }
}

impl CommandTool {
  pub(crate) fn execute(&self) -> ToolResult {
    let mut command = process::Command::new(&self.program);

    command.args(&self.arguments);

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    ToolResult::output(command.output())
  }
}

impl Display for CommandTool {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    if self.arguments.is_empty() {
      write!(f, "{}", self.program)
    } else {
      write!(f, "{} {}", self.program, self.arguments.join(" "))
    }
  }
}

impl ListFilesTool {
  pub(crate) fn execute(&self) -> ToolResult {
    let mut command = process::Command::new("rg");

    command.arg("--files");

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    ToolResult::output(command.output())
  }
}

impl ReadFileTool {
  pub(crate) fn execute(&self) -> ToolResult {
    match std::fs::read_to_string(&self.path) {
      Ok(content) => ToolResult::content(content),
      Err(error) => ToolResult::error(&error),
    }
  }
}

impl SearchFilesTool {
  pub(crate) fn execute(&self) -> ToolResult {
    let mut command = process::Command::new("rg");

    command.args(&self.arguments);

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    ToolResult::output(command.output())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  static TEMP_INDEX: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

  fn tempdir() -> PathBuf {
    let path = env::temp_dir().join(format!(
      "kotomori-tools-test-{}-{}",
      process::id(),
      TEMP_INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));

    let _ = std::fs::remove_dir_all(&path);

    std::fs::create_dir_all(&path).unwrap();

    path
  }

  #[test]
  fn command_execute() {
    assert_eq!(
      CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }
      .execute(),
      ToolResult {
        content: None,
        error: None,
        exit_status: Some(0),
        stdout: Some("bar\n".into()),
      },
    );
  }

  #[test]
  fn command_execute_error() {
    let result = CommandTool {
      arguments: vec!["-c".into(), "echo bar >&2; exit 42".into()],
      cwd: None,
      program: "sh".into(),
    }
    .execute();

    assert_eq!(
      result,
      ToolResult {
        content: None,
        error: Some("bar\n".into()),
        exit_status: Some(42),
        stdout: None,
      },
    );
  }

  #[test]
  fn list_files_execute() {
    let cwd = tempdir();

    std::fs::write(cwd.join("foo"), "bar").unwrap();
    std::fs::write(cwd.join("baz"), "qux").unwrap();

    let result = ListFilesTool { cwd: Some(cwd) }.execute();
    let stdout = result.stdout.unwrap();
    let mut files = stdout.lines().collect::<Vec<_>>();
    files.sort_unstable();

    assert_eq!(result.error, None);
    assert_eq!(result.exit_status, Some(0));
    assert_eq!(files, ["baz", "foo"]);
  }

  #[test]
  fn read_file_execute() {
    let cwd = tempdir();
    let path = cwd.join("foo");

    std::fs::write(&path, "bar").unwrap();

    assert_eq!(
      ReadFileTool { path }.execute(),
      ToolResult {
        content: Some("bar".into()),
        error: None,
        exit_status: None,
        stdout: None,
      },
    );
  }

  #[test]
  fn search_files_execute() {
    let cwd = tempdir();

    std::fs::write(cwd.join("foo"), "bar").unwrap();

    assert_eq!(
      SearchFilesTool {
        arguments: vec!["bar".into()],
        cwd: Some(cwd),
      }
      .execute(),
      ToolResult {
        content: None,
        error: None,
        exit_status: Some(0),
        stdout: Some("foo:bar\n".into()),
      },
    );
  }

  #[test]
  fn tool_parameters_are_derived_from_type() {
    let tool = TOOLS.iter().find(|tool| tool.name == "command").unwrap();

    assert_eq!(
      tool.parameters,
      json!({
        "type": "object",
        "properties": {
          "arguments": {
            "type": "array",
            "items": {"type": "string"},
          },
          "cwd": {"type": ["string", "null"]},
          "program": {"type": "string"},
        },
        "required": ["arguments", "program"],
        "additionalProperties": false,
      }),
    );
  }

  #[test]
  fn tool_invocation_kind_execute() {
    assert_eq!(
      ToolInvocationKind::CommandTool(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      })
      .execute(),
      ToolResult {
        content: None,
        error: None,
        exit_status: Some(0),
        stdout: Some("bar\n".into()),
      },
    );
  }
}
