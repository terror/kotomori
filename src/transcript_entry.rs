use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum TranscriptEntry {
  Agent(String),
  Error(String),
  Interrupted,
  Reasoning(String),
  Tool {
    invocation: ToolInvocation,
    result: Option<ToolResult>,
  },
  User(String),
}
