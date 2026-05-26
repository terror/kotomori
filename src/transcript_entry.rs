use super::*;

#[derive(Debug)]
pub(crate) enum TranscriptEntry {
  Agent(String),
  Interrupted,
  Tool {
    invocation: ToolInvocation,
    result: Option<ToolResult>,
  },
  User(String),
}

impl TranscriptEntry {
  pub(crate) fn messages(&self) -> Vec<Message> {
    match self {
      Self::Agent(content) => vec![Message::new(Role::Agent, content.clone())],
      Self::Interrupted => Vec::new(),
      Self::Tool { invocation, result } => result.as_ref().map_or_else(
        || vec![invocation.message()],
        |result| {
          vec![invocation.message(), result.message(invocation.id.clone())]
        },
      ),
      Self::User(content) => vec![Message::new(Role::User, content.clone())],
    }
  }
}
