use super::*;

macro_rules! define_tools {
  ($macro:ident) => {
    $macro! {
      ApplyPatch(ApplyPatchTool),
      Command(CommandTool),
      ReadFile(ReadFileTool),
      SearchFiles(SearchFilesTool),
    }
  };
}

macro_rules! impl_from_tools {
  ($( $variant:ident($tool:ty), )*) => {
    $(
      impl From<$tool> for ToolInvocationKind {
        fn from(tool: $tool) -> Self {
          Self::$variant(tool)
        }
      }
    )*
  };
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyPatchTool {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) patch: String,
}

#[async_trait]
impl ToolSpec for ApplyPatchTool {
  const DESCRIPTION: &'static str = "Apply a unified patch to the workspace.";
  const NAME: &'static str = "apply_patch";

  fn action(tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Completed => "Applied",
      ToolActionTense::Failed => "Failed applying",
      ToolActionTense::Progressive => "Applying",
    }
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    once(("patch", format!("{} lines", self.patch.lines().count())))
      .chain(
        self
          .cwd
          .as_ref()
          .map(|cwd| ("cwd", cwd.display().to_string())),
      )
      .collect()
  }

  fn display(&self) -> String {
    Self::NAME.into()
  }

  async fn execute(&self, executor: &Executor) -> ToolResult {
    let mut command = tokio::process::Command::new(Self::NAME);

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    executor.execute(command, Some(self.patch.clone())).await
  }

  fn requires_approval(&self) -> bool {
    true
  }

  fn subject(&self) -> String {
    Self::NAME.into()
  }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandTool {
  pub(crate) arguments: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) program: String,
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

#[async_trait]
impl ToolSpec for CommandTool {
  const DESCRIPTION: &'static str = "Run a command and capture stdout, stderr, and exit status. Do not use this to list or search project files; use search_files instead.";
  const NAME: &'static str = "command";

  fn action(tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Completed => "Ran",
      ToolActionTense::Failed => "Failed running",
      ToolActionTense::Progressive => "Running",
    }
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    self
      .cwd
      .as_ref()
      .map(|cwd| ("cwd", cwd.display().to_string()))
      .into_iter()
      .collect()
  }

  fn display(&self) -> String {
    self.to_string()
  }

  async fn execute(&self, executor: &Executor) -> ToolResult {
    let mut command = tokio::process::Command::new(&self.program);

    command.args(&self.arguments);

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    executor.execute(command, None).await
  }

  fn requires_approval(&self) -> bool {
    true
  }

  fn subject(&self) -> String {
    self.to_string()
  }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadFileTool {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) end_line: Option<usize>,
  pub(crate) path: PathBuf,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) start_line: Option<usize>,
}

#[async_trait]
impl ToolSpec for ReadFileTool {
  const DESCRIPTION: &'static str = "Read a UTF-8 text file. start_line and end_line are optional 1-based inclusive line numbers.";
  const NAME: &'static str = "read_file";

  fn action(tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Completed => "Read",
      ToolActionTense::Failed => "Failed reading",
      ToolActionTense::Progressive => "Reading",
    }
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    self
      .cwd
      .as_ref()
      .map(|cwd| ("cwd", cwd.display().to_string()))
      .into_iter()
      .collect()
  }

  fn display(&self) -> String {
    self
      .cwd
      .as_ref()
      .map_or(format!("read {}", self.path.display()), |cwd| {
        format!("read {} in {}", self.path.display(), cwd.display())
      })
  }

  async fn execute(&self, executor: &Executor) -> ToolResult {
    executor.read_file(self).await
  }

  fn subject(&self) -> String {
    self
      .cwd
      .as_ref()
      .map_or(self.path.display().to_string(), |cwd| {
        format!("{} in {}", self.path.display(), cwd.display())
      })
  }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchFilesTool {
  pub(crate) arguments: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) cwd: Option<PathBuf>,
}

#[async_trait]
impl ToolSpec for SearchFilesTool {
  const DESCRIPTION: &'static str =
    "Search or list project files with ripgrep. Use --files to list files.";
  const NAME: &'static str = "search_files";

  fn action(tense: ToolActionTense) -> &'static str {
    match tense {
      ToolActionTense::Completed => "Searched",
      ToolActionTense::Failed => "Failed searching",
      ToolActionTense::Progressive => "Searching",
    }
  }

  fn display(&self) -> String {
    if self.arguments.is_empty() {
      self.cwd.as_ref().map_or("search files".into(), |cwd| {
        format!("search files in {}", cwd.display())
      })
    } else if let Some(cwd) = &self.cwd {
      format!(
        "search files {} in {}",
        self.arguments.join(" "),
        cwd.display()
      )
    } else {
      format!("search files {}", self.arguments.join(" "))
    }
  }

  async fn execute(&self, executor: &Executor) -> ToolResult {
    let mut command = tokio::process::Command::new("rg");

    command.args(&self.arguments);

    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }

    executor.execute(command, None).await
  }

  fn requires_approval(&self) -> bool {
    self
      .arguments
      .iter()
      .any(|argument| argument == "--pre" || argument.starts_with("--pre="))
  }

  fn subject(&self) -> String {
    let query = if self.arguments.is_empty() {
      "files".into()
    } else {
      self.arguments.join(" ")
    };

    self
      .cwd
      .as_ref()
      .map_or(query.clone(), |cwd| format!("{query} in {}", cwd.display()))
  }
}

define_tools!(impl_from_tools);

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn tool_parameters_are_derived_from_type() {
    let tool = Tool::new::<CommandTool>();

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
}
