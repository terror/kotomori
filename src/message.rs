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

impl Component for Message {
  fn render(&self, width: u16) -> Vec<Line> {
    match self {
      Self::Agent(content) => content
        .iter()
        .flat_map(|content| match content {
          AgentMessageContent::Reasoning(reasoning) => reasoning
            .split('\n')
            .map(|line| {
              Line::from([Span::styled(format!(" {line}"), Style::DarkGray)])
            })
            .collect::<Vec<_>>(),
          AgentMessageContent::Text(text) => {
            text.split('\n').map(Line::raw).collect::<Vec<_>>()
          }
          AgentMessageContent::ToolCall(invocation) => {
            vec![Line::raw(invocation.to_string())]
          }
        })
        .collect(),
      Self::User(content) => content
        .iter()
        .flat_map(|content| match content {
          UserMessageContent::Text(text) => {
            FramedLines::raw(text.split('\n')).render(width)
          }
          UserMessageContent::ToolResult { result, .. } => result
            .output()
            .unwrap_or_default()
            .split('\n')
            .map(Line::raw)
            .collect::<Vec<_>>(),
        })
        .collect(),
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
                  invocation.arguments(),
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
      kind: ToolInvocationKind::ReadFile(ReadFileTool {
        cwd: None,
        end_line: None,
        path: "bar".into(),
        start_line: None,
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
            "read_file",
            json!({"path": "bar"})
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
      kind: ToolInvocationKind::ReadFile(ReadFileTool {
        cwd: None,
        end_line: None,
        path: "bar".into(),
        start_line: None,
      }),
    };

    let tool_use = RigMessage::from(&invocation.message());

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
