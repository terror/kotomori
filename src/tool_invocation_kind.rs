use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch { cwd: Option<PathBuf>, patch: String },
  Command(CommandInvocation),
  ListFiles(CommandInvocation),
  ReadFile { path: PathBuf },
  SearchFiles(CommandInvocation),
}
