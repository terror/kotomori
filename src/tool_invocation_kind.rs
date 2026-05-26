use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch(tools::ApplyPatchTool),
  Command(tools::CommandTool),
  ListFiles(tools::ListFilesTool),
  ReadFile(tools::ReadFileTool),
  SearchFiles(tools::SearchFilesTool),
}
