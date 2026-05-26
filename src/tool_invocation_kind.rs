use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum ToolInvocationKind {
  ApplyPatchTool(ApplyPatchTool),
  CommandTool(CommandTool),
  ListFilesTool(ListFilesTool),
  ReadFileTool(ReadFileTool),
  SearchFilesTool(SearchFilesTool),
}
