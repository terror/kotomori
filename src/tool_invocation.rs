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

  fn command(&self) -> Option<&tool::command::Command> {
    match &self.kind {
      ToolInvocationKind::Command(command) => Some(command),
      ToolInvocationKind::ApplyPatch(_)
      | ToolInvocationKind::ListFiles(_)
      | ToolInvocationKind::ReadFile(_)
      | ToolInvocationKind::SearchFiles(_) => None,
    }
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
