use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolInvocationKind {
  ApplyPatch(ApplyPatchTool),
  Command(CommandTool),
  ListFiles(ListFilesTool),
  ReadFile(ReadFileTool),
  SearchFiles(SearchFilesTool),
}
