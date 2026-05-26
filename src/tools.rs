use super::*;

macro_rules! define_tools {
  (
    $(
      $tool:ident => $variant:ident {
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
          Self::$variant(tool)
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
  ApplyPatchTool => ApplyPatch {
    name: "apply_patch",
    description: "Apply a unified patch to the workspace.",
    fields: {
      cwd: Option<PathBuf>,
      patch: String,
    },
  },
  CommandTool => Command {
    name: "command",
    description: "Run a command and capture stdout, stderr, and exit status. Do not use this to list project files; use list_files instead.",
    fields: {
      arguments: Vec<String>,
      cwd: Option<PathBuf>,
      program: String,
    },
  },
  ListFilesTool => ListFiles {
    name: "list_files",
    description: "List project files while respecting .gitignore and other standard ignore rules.",
    fields: {
      cwd: Option<PathBuf>,
    },
  },
  ReadFileTool => ReadFile {
    name: "read_file",
    description: "Read a UTF-8 text file.",
    fields: {
      path: PathBuf,
    },
  },
  SearchFilesTool => SearchFiles {
    name: "search_files",
    description: "Search files with ripgrep.",
    fields: {
      arguments: Vec<String>,
      cwd: Option<PathBuf>,
    },
  },
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
