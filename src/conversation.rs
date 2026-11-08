use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Conversation {
  messages: Vec<Message>,
}

impl Conversation {
  pub(crate) fn clear(&mut self) {
    self.messages.clear();
  }

  pub(crate) fn interrupt_pending_calls(&mut self) -> Vec<Message> {
    self.interrupt_pending_calls_with(&[])
  }

  fn interrupt_pending_calls_with(
    &mut self,
    content: &[UserMessageContent],
  ) -> Vec<Message> {
    let Some((index, Message::Agent(message))) = self
      .messages
      .iter()
      .enumerate()
      .rfind(|(_, message)| matches!(message, Message::Agent(_)))
    else {
      return Vec::new();
    };

    let messages = message
      .content
      .iter()
      .filter_map(|content| match content {
        AssistantContent::ToolCall(call) => Some(call),
        _ => None,
      })
      .filter(|call| {
        !self.messages[index + 1..]
          .iter()
          .filter_map(|message| match message {
            Message::User(content) => Some(content.as_slice()),
            Message::Agent(_) => None,
          })
          .flatten()
          .chain(content)
          .any(|content| content.is_tool_result_for(&call.id))
      })
      .map(|call| {
        Message::User(vec![UserMessageContent::ToolResult {
          call: call.id.clone(),
          name: call.function.name.clone(),
          result: ToolResult {
            stderr: Some("interrupted".into()),
            ..Default::default()
          },
        }])
      })
      .collect::<Vec<_>>();

    self.messages.extend(messages.iter().cloned());

    messages
  }

  pub(crate) fn last_response(&self) -> Option<String> {
    self
      .messages
      .iter()
      .rev()
      .find_map(|message| match message {
        Message::Agent(message) => message.text(),
        Message::User(_) => None,
      })
  }

  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn push(&mut self, message: Message) -> Vec<Message> {
    let messages = match &message {
      Message::Agent(_) => self.interrupt_pending_calls(),
      Message::User(content)
        if content
          .iter()
          .any(|content| matches!(content, UserMessageContent::Text(_))) =>
      {
        self.interrupt_pending_calls_with(content)
      }
      Message::User(_) => Vec::new(),
    };

    self.messages.push(message);

    messages
  }

  pub(crate) fn push_partial(
    &mut self,
    buffer: &MessageBuffer,
  ) -> Vec<Message> {
    if buffer.is_empty() {
      return Vec::new();
    }

    let message = buffer.message();

    if message.content.is_empty() {
      self.interrupt_pending_calls()
    } else {
      self.push(Message::Agent(message))
    }
  }
}

impl From<Vec<Message>> for Conversation {
  fn from(messages: Vec<Message>) -> Self {
    let mut conversation = Self::default();

    for message in messages {
      conversation.push(message);
    }

    conversation
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn messages_preserve_completed_content_and_boundaries() {
    let messages = vec![
      Message::User(vec![UserMessageContent::Text("foo".into())]),
      Message::Agent(
        vec![
          AssistantContent::text("bar"),
          AssistantContent::ToolCall(
            ToolInvocation::new(
              "foo",
              CommandArguments {
                command: "bar".into(),
                cwd: None,
              },
            )
            .protocol,
          ),
          AssistantContent::text("baz"),
        ]
        .into(),
      ),
      Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::new("foo"),
        name: ToolName::new("command").unwrap(),
        result: ToolResult::default(),
      }]),
      Message::Agent(vec![AssistantContent::reasoning("foo", "qux")].into()),
      Message::Agent(vec![AssistantContent::text("quux")].into()),
    ];

    let conversation = Conversation::from(messages.clone());

    assert_eq!(conversation.messages(), messages);
  }

  #[test]
  fn messages_skip_unfinished_reasoning() {
    let mut buffer = MessageBuffer::default();

    buffer.apply(MessageUpdate::ReasoningDelta {
      delta: "foo".into(),
      index: 0,
    });

    let mut conversation = Conversation::default();

    conversation.push_partial(&buffer);

    assert_eq!(conversation.messages(), []);
  }
}
