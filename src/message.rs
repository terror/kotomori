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

impl From<&Message> for RigMessage {
  fn from(message: &Message) -> Self {
    match &message.kind {
      MessageKind::Text {
        content,
        role: Role::Agent,
      } => Self::assistant(content.clone()),
      MessageKind::Text {
        content,
        role: Role::User,
      } => Self::user(content.clone()),
      MessageKind::ToolResult { content, id, .. } => {
        Self::tool_result(id.clone(), content.clone())
      }
      MessageKind::ToolUse {
        arguments,
        id,
        name,
      } => Self::Assistant {
        content: OneOrMany::one(AssistantContent::tool_call(
          id.clone(),
          name.clone(),
          arguments.clone(),
        )),
        id: None,
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn rig_tool_messages() {
    let tool_use = RigMessage::from(&Message::tool_use(
      "foo",
      "read_file",
      json!({"path": "bar"}),
    ));

    assert_eq!(
      tool_use,
      RigMessage::Assistant {
        content: OneOrMany::one(AssistantContent::tool_call(
          "foo",
          "read_file",
          json!({"path": "bar"}),
        ),),
        id: None,
      },
    );

    let tool_result =
      RigMessage::from(&Message::tool_result("foo", "bar", false));

    assert_eq!(tool_result, RigMessage::tool_result("foo", "bar"));
  }
}
