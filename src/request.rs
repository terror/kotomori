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

  #[cfg(test)]
  pub(crate) fn new(
    model: Model,
    messages: Vec<Message>,
    tool_registry: ToolRegistry,
  ) -> Self {
    Self {
      messages,
      model,
      system: None,
      tool_registry,
    }
  }

  pub(crate) fn with_system(
    model: Model,
    messages: Vec<Message>,
    system: impl Into<String>,
    tool_registry: ToolRegistry,
  ) -> Self {
    let system = system.into();

    let system = if system.is_empty() {
      None
    } else {
      Some(system)
    };

    Self {
      messages,
      model,
      system,
      tool_registry,
    }
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
  fn chat_messages() {
    let messages = vec![
      Message::Agent(vec![AgentMessageContent::Text("bar".into())]),
      Message::User(vec![UserMessageContent::Text("foo".into())]),
    ];

    let request = Request::new(
      Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      messages.clone(),
      ToolRegistry::new(Vec::new()),
    );

    assert_eq!(request.messages, messages);
    assert_eq!(request.model.name, "foo");
  }

  #[test]
  fn last_user_message() {
    let last_user_message =
      Message::User(vec![UserMessageContent::Text("baz".into())]);

    let request = Request::new(
      Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      vec![
        Message::Agent(vec![AgentMessageContent::Text("bar".into())]),
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        last_user_message.clone(),
      ],
      ToolRegistry::new(Vec::new()),
    );

    assert_eq!(request.last_user_message().unwrap(), &last_user_message);
  }

  #[test]
  fn rig_system_context() {
    let request = CompletionRequest::from(&Request::with_system(
      Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      vec![Message::User(vec![UserMessageContent::Text("bar".into())])],
      "baz",
      ToolRegistry::new(Vec::new()),
    ));

    assert_eq!(
      request.chat_history.iter().collect::<Vec<_>>(),
      vec![&RigMessage::system("baz"), &RigMessage::user("bar")],
    );
  }

  #[test]
  fn rig_tools() {
    let parameters = json!({"type": "object"});

    let request = CompletionRequest::from(&Request::new(
      Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      vec![Message::User(vec![UserMessageContent::Text("bar".into())])],
      ToolRegistry::new(vec![Tool {
        description: "bar",
        invocation: |_| unreachable!(),
        name: "foo",
        parameters: parameters.clone(),
      }]),
    ));

    assert_eq!(
      request.tools,
      vec![ToolDefinition {
        description: "bar".into(),
        name: "foo".into(),
        parameters,
      }],
    );
  }
}
