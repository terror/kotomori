use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyPatchTool {
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) patch: String,
}

impl From<ApplyPatchTool> for ToolInvocationKind {
  fn from(tool: ApplyPatchTool) -> Self {
    Self::ApplyPatch(tool)
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandTool {
  pub(crate) arguments: Vec<String>,
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) program: String,
}

impl From<CommandTool> for ToolInvocationKind {
  fn from(tool: CommandTool) -> Self {
    Self::Command(tool)
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListFilesTool {
  pub(crate) cwd: Option<PathBuf>,
}

impl From<ListFilesTool> for ToolInvocationKind {
  fn from(tool: ListFilesTool) -> Self {
    Self::ListFiles(tool)
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadFileTool {
  pub(crate) path: PathBuf,
}

impl From<ReadFileTool> for ToolInvocationKind {
  fn from(tool: ReadFileTool) -> Self {
    Self::ReadFile(tool)
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchFilesTool {
  pub(crate) arguments: Vec<String>,
  pub(crate) cwd: Option<PathBuf>,
}

impl From<SearchFilesTool> for ToolInvocationKind {
  fn from(tool: SearchFilesTool) -> Self {
    Self::SearchFiles(tool)
  }
}

pub(crate) static TOOLS: LazyLock<Vec<Tool>> = LazyLock::new(|| {
  vec![
    Tool {
      name: "apply_patch",
      description: "Apply a unified patch to the workspace.",
      invocation: ToolInvocation::from_raw::<ApplyPatchTool>,
      parameters: json!({
        "type": "object",
        "properties": {
          "patch": {"type": "string"},
          "cwd": {"type": ["string", "null"]},
        },
        "required": ["patch"],
        "additionalProperties": false
      }),
    },
    Tool {
      name: "command",
      description: "Run a command and capture stdout, stderr, and exit status. Do not use this to list project files; use list_files instead.",
      invocation: ToolInvocation::from_raw::<CommandTool>,
      parameters: json!({
        "type": "object",
        "properties": {
          "program": {"type": "string"},
          "arguments": {
            "type": "array",
            "items": {"type": "string"}
          },
          "cwd": {"type": ["string", "null"]},
        },
        "required": ["program", "arguments"],
        "additionalProperties": false
      }),
    },
    Tool {
      name: "list_files",
      description: "List project files while respecting .gitignore and other standard ignore rules.",
      invocation: ToolInvocation::from_raw::<ListFilesTool>,
      parameters: json!({
        "type": "object",
        "properties": {
          "cwd": {"type": ["string", "null"]},
        },
        "required": [],
        "additionalProperties": false
      }),
    },
    Tool {
      name: "read_file",
      description: "Read a UTF-8 text file.",
      invocation: ToolInvocation::from_raw::<ReadFileTool>,
      parameters: json!({
        "type": "object",
        "properties": {
          "path": {"type": "string"},
        },
        "required": ["path"],
        "additionalProperties": false
      }),
    },
    Tool {
      name: "search_files",
      description: "Search files with ripgrep.",
      invocation: ToolInvocation::from_raw::<SearchFilesTool>,
      parameters: json!({
        "type": "object",
        "properties": {
          "arguments": {
            "type": "array",
            "items": {"type": "string"}
          },
          "cwd": {"type": ["string", "null"]},
        },
        "required": ["arguments"],
        "additionalProperties": false
      }),
    },
  ]
});
