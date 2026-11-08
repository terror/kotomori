use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conversation {
  messages: Vec<Message>,
}

impl Conversation {
  pub fn clear(&mut self) {
    self.messages.clear();
  }

  pub fn interrupt_pending_calls(&mut self) -> Result<Vec<Message>> {
    let mut pending = Vec::<&ToolRequest>::new();

    for message in &self.messages {
      match message {
        Message::Agent(message) => {
          if !pending.is_empty() {
            bail!(
              "cannot recover unfinished tool calls from an earlier assistant turn"
            );
          }

          pending.extend(message.content.iter().filter_map(|content| {
            match content {
              AssistantContent::ToolCall(call) => Some(call),
              _ => None,
            }
          }));
        }
        Message::User(content) => pending.retain(|call| {
          !content
            .iter()
            .any(|content| content.is_tool_result_for(&call.id))
        }),
      }
    }

    let messages = pending
      .into_iter()
      .map(|call| {
        Message::User(vec![UserMessageContent::ToolResult {
          call: call.id.clone(),
          name: call.function.name.clone(),
          result: ToolResult {
            outcome: ToolOutcome::Interrupted,
            stderr: Some("interrupted".into()),
            ..Default::default()
          },
        }])
      })
      .collect::<Vec<_>>();

    self.messages.extend(messages.iter().cloned());

    Ok(messages)
  }

  #[must_use]
  pub fn last_response(&self) -> Option<String> {
    self
      .messages
      .iter()
      .rev()
      .find_map(|message| match message {
        Message::Agent(message) => message.text(),
        Message::User(_) => None,
      })
  }

  #[must_use]
  pub fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub fn push(&mut self, message: Message) {
    self.messages.push(message);
  }
}

impl From<Vec<Message>> for Conversation {
  fn from(messages: Vec<Message>) -> Self {
    Self { messages }
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
          AssistantContent::tool_call(
            "foo",
            ToolName::new("command").unwrap(),
            serde_json::json!({"command": "bar"}),
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
}
