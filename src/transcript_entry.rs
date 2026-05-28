use super::*;

#[derive(Debug)]
pub(crate) enum TranscriptEntry {
  Agent(String),
  Interrupted,
  Reasoning(String),
  Tool {
    invocation: ToolInvocation,
    result: Option<ToolResult>,
  },
  User(String),
}
