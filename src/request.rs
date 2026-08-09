use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Request {
  pub(crate) messages: Vec<Message>,
  pub(crate) model: Model,
  pub(crate) system: Option<String>,
  pub(crate) tool_registry: ToolRegistry,
}

impl Request {
  pub(crate) fn last_user_message(&self) -> Option<&Message> {
    self
      .messages
      .iter()
      .rev()
      .find(|message| message.user_content().is_some())
  }
}

impl From<&Request> for CompletionRequest {
  fn from(request: &Request) -> Self {
    let messages = request
      .system
      .as_deref()
      .map(RigMessage::system)
      .into_iter()
      .chain(request.messages.iter().map(Into::into))
      .collect::<Vec<_>>();

    let chat_history = if messages.is_empty() {
      OneOrMany::one(RigMessage::user(""))
    } else {
      match OneOrMany::many(messages) {
        Ok(messages) => messages,
        Err(_) => OneOrMany::one(RigMessage::user("")),
      }
    };

    Self {
      additional_params: None,
      chat_history,
      documents: Vec::new(),
      max_tokens: None,
      model: Some(request.model.name.clone()),
      output_schema: None,
      preamble: None,
      temperature: None,
      tool_choice: None,
      tools: request.tool_registry.tools.iter().map(Into::into).collect(),
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
      tool_registry: ToolRegistry::new(Vec::new()),
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
        Message::Agent(vec![AgentMessageContent::Text("qux".into())]),
      ],
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: Some("baz".into()),
      tool_registry: ToolRegistry::new(Vec::new()),
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
    let parameters = json!({"type": "object"});

    let request = CompletionRequest::from(&Request {
      messages: Vec::new(),
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: None,
      tool_registry: ToolRegistry::new(vec![Tool {
        description: "bar",
        invocation: |_| unreachable!(),
        name: "foo",
        parameters: parameters.clone(),
      }]),
    });

    assert_eq!(
      request.tools,
      vec![ToolDefinition {
        description: "bar".into(),
        name: "foo".into(),
        parameters,
      }],
    );
  }

  #[test]
  fn last_user_message_returns_latest_text_message() {
    let last_user_message =
      Message::User(vec![UserMessageContent::Text("baz".into())]);

    let request = Request {
      messages: vec![
        Message::Agent(vec![AgentMessageContent::Text("bar".into())]),
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        last_user_message.clone(),
        Message::Agent(vec![AgentMessageContent::Text("qux".into())]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "quux".into(),
          result: ToolResult::default(),
        }]),
      ],
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: None,
      tool_registry: ToolRegistry::new(Vec::new()),
    };

    assert_eq!(request.last_user_message().unwrap(), &last_user_message);
  }

  #[test]
  fn last_user_message_returns_none_without_user_text() {
    let request = Request {
      messages: vec![
        Message::Agent(vec![AgentMessageContent::Text("foo".into())]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "bar".into(),
          result: ToolResult::default(),
        }]),
      ],
      model: Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: None,
      tool_registry: ToolRegistry::new(Vec::new()),
    };

    assert_eq!(request.last_user_message(), None);
  }
}
