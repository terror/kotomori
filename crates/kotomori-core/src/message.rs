use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Message {
  Agent(AgentMessage),
  User(Vec<UserMessageContent>),
}

impl Message {
  #[must_use]
  pub fn has_tool_result(&self) -> bool {
    match self {
      Self::Agent(_) => false,
      Self::User(content) => content.iter().any(|content| {
        matches!(content, UserMessageContent::ToolResult { .. })
      }),
    }
  }

  pub fn user_content(&self) -> Option<&str> {
    match self {
      Self::User(content) => content.iter().find_map(UserMessageContent::text),
      Self::Agent(_) => None,
    }
  }
}
