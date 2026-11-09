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
    let messages = Self::pending_calls(&self.messages)?
      .into_iter()
      .map(|call| {
        Message::User(vec![UserMessageContent::ToolResult {
          call: call.id.clone(),
          name: call.function.name.clone(),
          result: ToolResult {
            content: "interrupted".into(),
            outcome: ToolOutcome::Interrupted,
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

  pub(super) fn pending_calls(
    messages: &[Message],
  ) -> Result<Vec<&ToolRequest>> {
    let mut pending = Vec::<&ToolRequest>::new();

    for message in messages {
      match message {
        Message::Agent(message) => {
          if !pending.is_empty() {
            bail!(
              "cannot recover unfinished tool calls from an earlier assistant turn"
            );
          }

          for content in &message.content {
            if let AssistantContent::ToolCall(call) = content {
              call.id.validate()?;

              ensure!(
                !pending.iter().any(|pending| {
                  pending.id.correlation() == call.id.correlation()
                }),
                "duplicate tool call in conversation"
              );

              pending.push(call);
            }
          }
        }
        Message::User(content) => {
          for content in content {
            match content {
              UserMessageContent::Text(_) => ensure!(
                pending.is_empty(),
                "unfinished tool calls must be resolved before user text"
              ),
              UserMessageContent::ToolResult { call, name, .. } => {
                call.validate()?;

                let index = pending
                  .iter()
                  .position(|pending| {
                    pending.id == *call && pending.function.name == *name
                  })
                  .context(
                    "tool result without a matching call in conversation",
                  )?;

                pending.remove(index);
              }
            }
          }
        }
      }
    }

    Ok(pending)
  }

  pub fn push(&mut self, message: Message) {
    self.messages.push(message);
  }

  pub fn validate(&self) -> Result {
    Self::pending_calls(&self.messages).map(|_| ())
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
  fn exchange_validation_is_shared_with_context() {
    #[track_caller]
    fn case(calls: &[CallId], expected: Option<&str>) {
      let messages = vec![
        Message::Agent(
          calls
            .iter()
            .map(|id| {
              AssistantContent::ToolCall(ToolRequest::new(
                id.clone(),
                ToolFunction {
                  arguments: Value::Null,
                  name: ToolName::new("foo").unwrap(),
                },
              ))
            })
            .collect::<Vec<_>>()
            .into(),
        ),
        Message::User(
          calls
            .iter()
            .map(|call| UserMessageContent::ToolResult {
              call: call.clone(),
              name: ToolName::new("foo").unwrap(),
              result: ToolResult::default(),
            })
            .collect(),
        ),
      ];

      for result in [
        Conversation::from(messages.clone()).validate(),
        Request {
          generation: &GenerationOptions::default(),
          messages: &messages,
          model: &Model::default(),
          system: None,
          tools: &[],
        }
        .validate(),
      ] {
        assert_eq!(
          result.err().map(|error| error.to_string()).as_deref(),
          expected
        );
      }
    }

    case(
      &[CallId::new("foo"), CallId::Local(Uuid::nil().to_string())],
      None,
    );
    case(
      &[CallId::new("")],
      Some("provider tool call identity must not be empty"),
    );
    case(
      &[CallId::Local("foo".into())],
      Some("local tool call identity must be a canonical UUID"),
    );
    case(
      &["bar", "baz"].map(|item_id| CallId::Provider {
        call_id: "foo".into(),
        item_id: Some(item_id.into()),
      }),
      Some("duplicate tool call in conversation"),
    );
  }

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
