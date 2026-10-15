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

impl From<&Message> for RigMessage {
  fn from(message: &Message) -> Self {
    match message {
      Message::Agent(message) => Self::Assistant {
        content: message.content.clone(),
        id: message.id.clone(),
      },
      Message::User(content) => Self::User {
        content: content
          .iter()
          .map(|content| match content {
            UserMessageContent::Text(text) => UserContent::text(text.clone()),
            UserMessageContent::ToolResult { call, name, result } => {
              UserContent::tool_result(
                call.clone(),
                name.clone(),
                vec![ToolResultContent::text(result.message_content())],
              )
            }
          })
          .collect(),
      },
    }
  }
}
