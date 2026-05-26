use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageKind {
  Text {
    content: String,
    role: Role,
  },
  ToolResult {
    content: String,
    id: String,
    is_error: bool,
  },
  ToolUse {
    arguments: Value,
    id: String,
    name: String,
  },
}
