use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
  Agent(Vec<AgentMessageContent>),
  User(Vec<UserMessageContent>),
}

impl Message {
  pub(crate) fn content(&self) -> Option<&str> {
    match self {
      Self::Agent(content) => {
        content.iter().find_map(AgentMessageContent::text)
      }
      Self::User(content) => content.iter().find_map(UserMessageContent::text),
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
      Message::Agent(content) => Self::Assistant {
        content: OneOrMany::many(
          content
            .iter()
            .map(|content| match content {
              AgentMessageContent::Reasoning(reasoning) => {
                AssistantContent::reasoning(reasoning.clone())
              }
              AgentMessageContent::Text(text) => {
                AssistantContent::text(text.clone())
              }
              AgentMessageContent::ToolCall(invocation) => {
                AssistantContent::tool_call(
                  invocation.id.clone(),
                  invocation.kind.name(),
                  invocation.kind.arguments(),
                )
              }
            })
            .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| OneOrMany::one(AssistantContent::text(""))),
        id: None,
      },
      Message::User(content) => Self::User {
        content: OneOrMany::many(
          content
            .iter()
            .map(|content| match content {
              UserMessageContent::Text(text) => UserContent::text(text.clone()),
              UserMessageContent::ToolResult { id, result } => {
                UserContent::tool_result(
                  id.clone(),
                  OneOrMany::one(ToolResultContent::text(
                    result.message_content(),
                  )),
                )
              }
            })
            .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| OneOrMany::one(UserContent::text(String::new()))),
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn rig_ordered_agent_content() {
    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }),
    };

    let message = Message::Agent(vec![
      AgentMessageContent::Reasoning("qux".into()),
      AgentMessageContent::Text("foo".into()),
      AgentMessageContent::ToolCall(invocation),
      AgentMessageContent::Text("baz".into()),
    ]);

    assert_eq!(
      RigMessage::from(&message),
      RigMessage::Assistant {
        content: OneOrMany::many(vec![
          AssistantContent::reasoning("qux"),
          AssistantContent::text("foo"),
          AssistantContent::tool_call(
            "foo",
            "command",
            json!({"arguments": ["bar"], "program": "echo"})
          ),
          AssistantContent::text("baz"),
        ])
        .unwrap(),
        id: None,
      },
    );
  }

  #[test]
  fn rig_tool_messages() {
    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["bar".into()],
        cwd: None,
        program: "echo".into(),
      }),
    };

    let tool_use =
      RigMessage::from(&Message::Agent(vec![AgentMessageContent::ToolCall(
        invocation,
      )]));

    assert_eq!(
      tool_use,
      RigMessage::Assistant {
        content: OneOrMany::one(AssistantContent::tool_call(
          "foo",
          "command",
          json!({"arguments": ["bar"], "program": "echo"}),
        ),),
        id: None,
      },
    );

    let tool_result =
      RigMessage::from(&ToolResult::content("bar").message("foo"));

    assert_eq!(
      tool_result,
      RigMessage::tool_result(
        "foo",
        serde_json::to_string(&ToolResult::content("bar")).unwrap()
      )
    );
  }
}
