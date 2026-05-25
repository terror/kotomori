use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
  pub(crate) content: String,
  pub(crate) role: Role,
}

impl Message {
  pub(crate) fn new(role: Role, content: impl Into<String>) -> Self {
    Self {
      content: content.into(),
      role,
    }
  }
}

impl Component for Message {
  fn render(&self, width: u16) -> Vec<Line> {
    match self.role {
      Role::Agent => self.content.split('\n').map(Line::raw).collect(),
      Role::User => FramedLines::raw(self.content.split('\n')).render(width),
    }
  }
}

impl From<&Message> for ChatCompletionRequestMessage {
  fn from(message: &Message) -> Self {
    match message.role {
      Role::Agent => ChatCompletionRequestMessage::Assistant(
        ChatCompletionRequestAssistantMessage {
          content: Some(ChatCompletionRequestAssistantMessageContent::Text(
            message.content.clone(),
          )),
          ..Default::default()
        },
      ),
      Role::User => {
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
          content: ChatCompletionRequestUserMessageContent::Text(
            message.content.clone(),
          ),
          name: None,
        })
      }
    }
  }
}

impl From<&Message> for types::MessageParam {
  fn from(message: &Message) -> Self {
    types::MessageParam {
      content: types::MessageContent::Text(message.content.clone()),
      role: match message.role {
        Role::Agent => types::Role::Assistant,
        Role::User => types::Role::User,
      },
    }
  }
}
