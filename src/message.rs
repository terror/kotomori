use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
  kind: MessageKind,
}

impl Message {
  pub(crate) fn content(&self) -> Option<&str> {
    match &self.kind {
      MessageKind::Text { content, .. }
      | MessageKind::ToolResult { content, .. } => Some(content),
      MessageKind::ToolUse { .. } => None,
    }
  }

  pub(crate) fn new(role: Role, content: impl Into<String>) -> Self {
    Self {
      kind: MessageKind::Text {
        content: content.into(),
        role,
      },
    }
  }

  #[cfg(test)]
  pub(crate) fn role(&self) -> Role {
    match &self.kind {
      MessageKind::Text { role, .. } => *role,
      MessageKind::ToolResult { .. } => Role::User,
      MessageKind::ToolUse { .. } => Role::Agent,
    }
  }

  pub(crate) fn tool_result(
    id: impl Into<String>,
    content: impl Into<String>,
    is_error: bool,
  ) -> Self {
    Self {
      kind: MessageKind::ToolResult {
        content: content.into(),
        id: id.into(),
        is_error,
      },
    }
  }

  pub(crate) fn tool_use(
    id: impl Into<String>,
    name: impl Into<String>,
    arguments: Value,
  ) -> Self {
    Self {
      kind: MessageKind::ToolUse {
        arguments,
        id: id.into(),
        name: name.into(),
      },
    }
  }

  pub(crate) fn user_content(&self) -> Option<&str> {
    match &self.kind {
      MessageKind::Text {
        content,
        role: Role::User,
      } => Some(content),
      MessageKind::Text { .. }
      | MessageKind::ToolResult { .. }
      | MessageKind::ToolUse { .. } => None,
    }
  }
}

impl From<RawToolCall> for Message {
  fn from(call: RawToolCall) -> Self {
    Self::tool_use(call.id, call.name, call.arguments)
  }
}

impl Component for Message {
  fn render(&self, width: u16) -> Vec<Line> {
    match &self.kind {
      MessageKind::Text {
        content,
        role: Role::Agent,
      } => content.split('\n').map(Line::raw).collect(),
      MessageKind::Text {
        content,
        role: Role::User,
      } => FramedLines::raw(content.split('\n')).render(width),
      MessageKind::ToolResult { content, .. } => {
        content.split('\n').map(Line::raw).collect()
      }
      MessageKind::ToolUse {
        arguments, name, ..
      } => vec![Line::raw(format!("{name} {arguments}"))],
    }
  }
}

impl From<&Message> for openai::ChatCompletionRequestMessage {
  fn from(message: &Message) -> Self {
    match &message.kind {
      MessageKind::Text {
        content,
        role: Role::Agent,
      } => openai::ChatCompletionRequestMessage::Assistant(
        openai::ChatCompletionRequestAssistantMessage {
          content: Some(
            openai::ChatCompletionRequestAssistantMessageContent::Text(
              content.clone(),
            ),
          ),
          ..Default::default()
        },
      ),
      MessageKind::Text {
        content,
        role: Role::User,
      } => openai::ChatCompletionRequestMessage::User(
        openai::ChatCompletionRequestUserMessage {
          content: openai::ChatCompletionRequestUserMessageContent::Text(
            content.clone(),
          ),
          name: None,
        },
      ),
      MessageKind::ToolResult { content, id, .. } => {
        openai::ChatCompletionRequestMessage::Tool(
          openai::ChatCompletionRequestToolMessage {
            content: openai::ChatCompletionRequestToolMessageContent::Text(
              content.clone(),
            ),
            tool_call_id: id.clone(),
          },
        )
      }
      MessageKind::ToolUse {
        arguments,
        id,
        name,
      } => openai::ChatCompletionRequestMessage::Assistant(
        openai::ChatCompletionRequestAssistantMessage {
          tool_calls: Some(vec![
            openai::ChatCompletionMessageToolCalls::Function(
              openai::ChatCompletionMessageToolCall {
                function: openai::FunctionCall {
                  arguments: serde_json::to_string(arguments)
                    .expect("failed to serialize tool arguments"),
                  name: name.clone(),
                },
                id: id.clone(),
              },
            ),
          ]),
          ..Default::default()
        },
      ),
    }
  }
}

impl From<&Message> for anthropic::MessageParam {
  fn from(message: &Message) -> Self {
    match &message.kind {
      MessageKind::Text { content, role } => anthropic::MessageParam {
        content: anthropic::MessageContent::Text(content.clone()),
        role: match role {
          Role::Agent => anthropic::Role::Assistant,
          Role::User => anthropic::Role::User,
        },
      },
      MessageKind::ToolResult {
        content,
        id,
        is_error,
      } => anthropic::MessageParam {
        content: anthropic::MessageContent::Blocks(vec![
          anthropic::ContentBlockParam::ToolResult {
            content: Some(content.clone()),
            is_error: Some(*is_error),
            tool_use_id: id.clone(),
          },
        ]),
        role: anthropic::Role::User,
      },
      MessageKind::ToolUse {
        arguments,
        id,
        name,
      } => anthropic::MessageParam {
        content: anthropic::MessageContent::Blocks(vec![
          anthropic::ContentBlockParam::ToolUse {
            id: id.clone(),
            input: arguments.clone(),
            name: name.clone(),
          },
        ]),
        role: anthropic::Role::Assistant,
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anthropic_tool_messages() {
    assert_eq!(
      serde_json::to_value(anthropic::MessageParam::from(&Message::tool_use(
        "foo",
        "read_file",
        json!({"path": "bar"})
      ),))
      .unwrap(),
      json!({
        "role": "assistant",
        "content": [
          {
            "type": "tool_use",
            "id": "foo",
            "name": "read_file",
            "input": {"path": "bar"},
          },
        ],
      }),
    );

    assert_eq!(
      serde_json::to_value(anthropic::MessageParam::from(
        &Message::tool_result("foo", "bar", false),
      ))
      .unwrap(),
      json!({
        "role": "user",
        "content": [
          {
            "type": "tool_result",
            "tool_use_id": "foo",
            "content": "bar",
            "is_error": false,
          },
        ],
      }),
    );
  }

  #[test]
  fn openai_tool_messages() {
    let tool_use = openai::ChatCompletionRequestMessage::from(
      &Message::tool_use("foo", "read_file", json!({"path": "bar"})),
    );

    assert_eq!(
      tool_use,
      openai::ChatCompletionRequestMessage::Assistant(
        openai::ChatCompletionRequestAssistantMessage {
          tool_calls: Some(vec![
            openai::ChatCompletionMessageToolCalls::Function(
              openai::ChatCompletionMessageToolCall {
                function: openai::FunctionCall {
                  arguments: r#"{"path":"bar"}"#.into(),
                  name: "read_file".into(),
                },
                id: "foo".into(),
              },
            ),
          ]),
          ..Default::default()
        },
      ),
    );

    let tool_result = openai::ChatCompletionRequestMessage::from(
      &Message::tool_result("foo", "bar", false),
    );

    assert_eq!(
      tool_result,
      openai::ChatCompletionRequestMessage::Tool(
        openai::ChatCompletionRequestToolMessage {
          content: openai::ChatCompletionRequestToolMessageContent::Text(
            "bar".into(),
          ),
          tool_call_id: "foo".into(),
        },
      ),
    );
  }
}
