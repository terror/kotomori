use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch {
    cwd: Option<PathBuf>,
    patch: String,
  },
  Command(CommandInvocation),
  ListFiles {
    cwd: Option<PathBuf>,
  },
  ReadFile {
    path: PathBuf,
  },
  SearchFiles {
    arguments: Vec<String>,
    cwd: Option<PathBuf>,
  },
}
