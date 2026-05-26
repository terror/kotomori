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
