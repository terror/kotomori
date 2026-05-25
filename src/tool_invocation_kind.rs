use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch(tool::apply_patch::ApplyPatch),
  Command(tool::command::Command),
  ListFiles(tool::list_files::ListFiles),
  ReadFile(tool::read_file::ReadFile),
  SearchFiles(tool::search_files::SearchFiles),
}
