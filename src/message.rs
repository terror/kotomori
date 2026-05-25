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

impl From<&Message> for openai::types::chat::ChatCompletionRequestMessage {
  fn from(message: &Message) -> Self {
    match message.role {
      Role::Agent => {
        openai::types::chat::ChatCompletionRequestMessage::Assistant(
          openai::types::chat::ChatCompletionRequestAssistantMessage {
            content: Some(
              openai::types::chat::ChatCompletionRequestAssistantMessageContent::Text(
                message.content.clone(),
              ),
            ),
            ..Default::default()
          },
        )
      }
      Role::User => {
        openai::types::chat::ChatCompletionRequestMessage::User(
          openai::types::chat::ChatCompletionRequestUserMessage {
            content:
              openai::types::chat::ChatCompletionRequestUserMessageContent::Text(
                message.content.clone(),
              ),
            name: None,
          },
        )
      }
    }
  }
}

impl From<&Message> for anthropic::types::MessageParam {
  fn from(message: &Message) -> Self {
    anthropic::types::MessageParam {
      content: anthropic::types::MessageContent::Text(message.content.clone()),
      role: match message.role {
        Role::Agent => anthropic::types::Role::Assistant,
        Role::User => anthropic::types::Role::User,
      },
    }
  }
}
