use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum Message {
  Agent(AgentMessage),
  User(Vec<UserMessageContent>),
}

impl Message {
  #[cfg(test)]
  pub(crate) fn agent(content: Vec<AssistantContent>) -> Self {
    Self::Agent(content.into())
  }

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

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn rig_agent_protocol_survives_serialization() {
    let content = vec![
      AssistantContent::Reasoning(
        Reasoning::new_with_signature("foo", Some("bar".into()))
          .with_id("baz".into())
          .sealed("foo"),
      ),
      AssistantContent::text("qux"),
      AssistantContent::ToolCall(
        ::rig::message::ToolCall::from_dual_wire(
          "quux",
          "quuz",
          ToolFunction {
            arguments: json!({"foo": null, "bar": ["baz"]}),
            name: ToolName::new("qux").unwrap(),
          },
        )
        .with_signature(Some("corge".into()))
        .with_additional_params(Some(json!({"foo": "bar"}))),
      ),
      AssistantContent::Reasoning(
        Reasoning::encrypted("grault")
          .with_id("garply".into())
          .sealed("foo"),
      ),
      AssistantContent::text("waldo"),
    ];

    let message = Message::Agent(AgentMessage {
      content: content.clone(),
      id: Some("fred".into()),
    });

    let message =
      serde_json::from_value::<Message>(serde_json::to_value(message).unwrap())
        .unwrap();

    assert_eq!(
      RigMessage::from(&message),
      RigMessage::Assistant {
        content,
        id: Some("fred".into()),
      },
    );
  }
}
