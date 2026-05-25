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
        ToolInvocationKind::ListFiles { .. } => "Listing",
        ToolInvocationKind::ReadFile { .. } => "Reading",
        ToolInvocationKind::SearchFiles { .. } => "Searching",
        _ => "Running",
      },
    }
  }

  fn command(&self) -> Option<&CommandInvocation> {
    match &self.kind {
      ToolInvocationKind::Command(command) => Some(command),
      ToolInvocationKind::ApplyPatch { .. }
      | ToolInvocationKind::ListFiles { .. }
      | ToolInvocationKind::ReadFile { .. }
      | ToolInvocationKind::SearchFiles { .. } => None,
    }
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn subject(&self) -> String {
    match &self.kind {
      ToolInvocationKind::ApplyPatch { .. } => "apply_patch".into(),
      ToolInvocationKind::Command(_) => self
        .command()
        .map_or_else(|| "command".into(), ToString::to_string),
      ToolInvocationKind::ListFiles { cwd } => cwd.as_ref().map_or_else(
        || "files".into(),
        |cwd| format!("files in {}", cwd.display()),
      ),
      ToolInvocationKind::ReadFile { path } => path.display().to_string(),
      ToolInvocationKind::SearchFiles { arguments, cwd } => {
        let query = if arguments.is_empty() {
          "files".into()
        } else {
          arguments.join(" ")
        };

        cwd
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
      ToolInvocationKind::ApplyPatch { .. } => write!(f, "apply_patch"),
      ToolInvocationKind::Command(_) => {
        if let Some(command) = self.command() {
          write!(f, "{command}")
        } else {
          write!(f, "command")
        }
      }
      ToolInvocationKind::ListFiles { cwd } => {
        if let Some(cwd) = cwd {
          write!(f, "list files in {}", cwd.display())
        } else {
          write!(f, "list files")
        }
      }
      ToolInvocationKind::ReadFile { path } => {
        write!(f, "read {}", path.display())
      }
      ToolInvocationKind::SearchFiles { arguments, cwd } => {
        if arguments.is_empty() {
          if let Some(cwd) = cwd {
            write!(f, "search files in {}", cwd.display())
          } else {
            write!(f, "search files")
          }
        } else if let Some(cwd) = cwd {
          write!(
            f,
            "search files {} in {}",
            arguments.join(" "),
            cwd.display()
          )
        } else {
          write!(f, "search files {}", arguments.join(" "))
        }
      }
    }
  }
}
