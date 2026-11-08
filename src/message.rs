use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum Message {
  Agent(AgentMessage),
  User(Vec<UserMessageContent>),
}

impl Message {
  pub(crate) fn has_tool_result(&self) -> bool {
    match self {
      Self::Agent(_) => false,
      Self::User(content) => content.iter().any(|content| {
        matches!(content, UserMessageContent::ToolResult { .. })
      }),
    }
  }

  pub(crate) fn user_content(&self) -> Option<&str> {
    match self {
      Self::User(content) => content.iter().find_map(UserMessageContent::text),
      Self::Agent(_) => None,
    }
  }
}
