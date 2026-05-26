use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[allow(clippy::enum_variant_names)]
#[serde(untagged)]
pub(crate) enum ToolInvocationKind {
  ApplyPatchTool(ApplyPatchTool),
  CommandTool(CommandTool),
  ListFilesTool(ListFilesTool),
  ReadFileTool(ReadFileTool),
  SearchFilesTool(SearchFilesTool),
}

impl ToolInvocationKind {
  pub(crate) fn arguments(&self) -> Value {
    serde_json::to_value(self).expect("failed to serialize tool arguments")
  }

  pub(crate) fn execute(&self) -> ToolResult {
    match self {
      Self::ApplyPatchTool(tool) => tool.execute(),
      Self::CommandTool(tool) => tool.execute(),
      Self::ListFilesTool(tool) => tool.execute(),
      Self::ReadFileTool(tool) => tool.execute(),
      Self::SearchFilesTool(tool) => tool.execute(),
    }
  }
}
