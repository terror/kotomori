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
        content: OneOrMany::many(message.content.clone())
          .unwrap_or_else(|_| OneOrMany::one(AssistantContent::text(""))),
        id: message.id.clone(),
      },
      Message::User(content) => Self::User {
        content: OneOrMany::many(content.iter().map(|content| match content {
          UserMessageContent::Text(text) => UserContent::text(text.clone()),
          UserMessageContent::ToolResult {
            call_id,
            id,
            result,
          } => UserContent::ToolResult(::rig::message::ToolResult {
            call_id: call_id.clone(),
            id: id.clone(),
            content: OneOrMany::one(ToolResultContent::text(
              result.message_content(),
            )),
          }),
        }))
        .unwrap_or_else(|_| OneOrMany::one(UserContent::text(String::new()))),
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
          .with_id("baz".into()),
      ),
      AssistantContent::text("qux"),
      AssistantContent::ToolCall(
        ::rig::message::ToolCall::new(
          "quux".into(),
          ToolFunction {
            arguments: json!({"foo": null, "bar": ["baz"]}),
            name: "qux".into(),
          },
        )
        .with_call_id("quuz".into())
        .with_signature(Some("corge".into()))
        .with_additional_params(Some(json!({"foo": "bar"}))),
      ),
      AssistantContent::Reasoning(
        Reasoning::encrypted("grault").with_id("garply".into()),
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
        content: OneOrMany::many(content).unwrap(),
        id: Some("fred".into()),
      },
    );
  }

  #[test]
  fn rig_tool_result() {
    let result = ToolResult {
      content: Some("bar".into()),
      ..Default::default()
    };

    let message = Message::User(vec![UserMessageContent::ToolResult {
      call_id: Some("baz".into()),
      id: "foo".into(),
      result: result.clone(),
    }]);

    assert_eq!(
      RigMessage::from(&message),
      RigMessage::tool_result_with_call_id(
        "foo",
        Some("baz".into()),
        result.message_content()
      )
    );
  }
}
