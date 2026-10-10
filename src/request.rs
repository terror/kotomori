use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Request {
  pub(crate) messages: Vec<Message>,
  pub(crate) model: Model,
  pub(crate) system: Option<String>,
  pub(crate) tools: Vec<ToolDefinition>,
}

impl Request {
  pub(crate) fn last_user_text(&self) -> Option<&str> {
    self.messages.iter().rev().find_map(Message::user_content)
  }
}

impl From<&Request> for CompletionRequest {
  fn from(request: &Request) -> Self {
    let chat_history = request
      .system
      .as_deref()
      .map(RigMessage::system)
      .into_iter()
      .chain(request.messages.iter().map(Into::into))
      .collect::<Vec<_>>();

    Self {
      additional_params: None,
      chat_history: if chat_history.is_empty() {
        vec![RigMessage::user("")]
      } else {
        chat_history
      },
      documents: Vec::new(),
      max_tokens: None,
      model: Some(request.model.name.clone()),
      output_schema: None,
      record_telemetry_content: false,
      temperature: None,
      tool_choice: None,
      tools: request.tools.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn completion_request_uses_blank_user_message_for_empty_history() {
    let request = CompletionRequest::from(&Request {
      messages: Vec::new(),
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: None,
      tools: Vec::new(),
    });

    assert_eq!(
      request.chat_history.iter().collect::<Vec<_>>(),
      vec![&RigMessage::user("")],
    );
  }

  #[test]
  fn completion_request_uses_system_context_and_model() {
    let request = CompletionRequest::from(&Request {
      messages: vec![
        Message::User(vec![UserMessageContent::Text("bar".into())]),
        Message::agent(vec![AssistantContent::text("qux")]),
      ],
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: Some("baz".into()),
      tools: Vec::new(),
    });

    assert_eq!(request.model.as_deref(), Some("foo"));

    assert_eq!(
      request.chat_history.iter().collect::<Vec<_>>(),
      vec![
        &RigMessage::system("baz"),
        &RigMessage::user("bar"),
        &RigMessage::assistant("qux"),
      ],
    );
  }

  #[test]
  fn completion_request_uses_tools() {
    #[track_caller]
    fn case(tools: &[ToolDefinition]) {
      let request = CompletionRequest::from(&Request {
        messages: Vec::new(),
        model: Model {
          name: "foo".into(),
          provider: "mock".into(),
        },
        system: None,
        tools: tools.to_vec(),
      });

      assert_eq!(request.tools, tools);
    }

    case(&[]);
    case(&[ToolDefinition {
      description: "bar".into(),
      name: "foo".into(),
      parameters: json!({
        "type": "object",
        "properties": {"baz": {"type": "string"}},
        "required": ["baz"],
      }),
    }]);
  }

  #[test]
  fn last_user_text_returns_none_without_user_text() {
    let request = Request {
      messages: vec![
        Message::agent(vec![AssistantContent::text("foo")]),
        Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::from_wire("bar"),
          name: ToolName::new("baz").unwrap(),
          result: ToolResult::default(),
        }]),
      ],
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: None,
      tools: Vec::new(),
    };

    assert_eq!(request.last_user_text(), None);
  }
}
