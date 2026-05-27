use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Request {
  messages: Vec<Message>,
  model: Model,
  system: Option<String>,
  tool_registry: ToolRegistry,
}

impl Request {
  pub(crate) fn last_user_message(&self) -> Option<&Message> {
    self
      .messages()
      .rev()
      .find(|message| message.user_content().is_some())
  }

  pub(crate) fn messages(&self) -> impl DoubleEndedIterator<Item = &Message> {
    self.messages.iter()
  }

  pub(crate) fn model(&self) -> &Model {
    &self.model
  }

  pub(crate) fn model_name(&self) -> &str {
    self.model.name()
  }

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

  pub(crate) fn system(&self) -> Option<&str> {
    self.system.as_deref()
  }

  pub(crate) fn tools(&self) -> impl Iterator<Item = &Tool> {
    self.tool_registry.tools()
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
      .system()
      .map(RigMessage::system)
      .into_iter()
      .chain(request.messages().map(Into::into))
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
      model: Some(request.model_name().into()),
      output_schema: None,
      preamble: None,
      temperature: None,
      tool_choice: None,
      tools: request.tools().map(Into::into).collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn chat_messages() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
      ],
      ToolRegistry::default(),
    );

    assert_eq!(request.model_name(), "foo");

    assert_eq!(
      request
        .messages()
        .map(|message| (message.role(), message.content().unwrap()))
        .collect::<Vec<_>>(),
      vec![(Role::User, "foo"), (Role::Agent, "bar")],
    );
  }

  #[test]
  fn last_user_message() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
        Message::new(Role::User, "baz"),
      ],
      ToolRegistry::default(),
    );

    assert_eq!(
      request.last_user_message().unwrap().content().unwrap(),
      "baz"
    );
  }

  #[test]
  fn rig_system_context() {
    let request = CompletionRequest::from(&Request::with_system(
      "fake:foo".parse().unwrap(),
      vec![Message::new(Role::User, "bar")],
      "baz",
      ToolRegistry::default(),
    ));

    assert_eq!(
      request.chat_history.iter().collect::<Vec<_>>(),
      vec![&RigMessage::system("baz"), &RigMessage::user("bar")],
    );
  }

  #[test]
  fn rig_tools() {
    let request = CompletionRequest::from(&Request::new(
      "fake:foo".parse().unwrap(),
      vec![Message::new(Role::User, "bar")],
      ToolRegistry::new(vec![Tool::new::<ReadFileTool>()]),
    ));

    assert_eq!(
      serde_json::to_value(request.tools).unwrap(),
      json!([
        {
          "name": "read_file",
          "description": "Read a UTF-8 text file. start_line and end_line are optional 1-based inclusive line numbers.",
          "parameters": {
            "additionalProperties": false,
            "properties": {
              "cwd": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "end_line": {
                "format": "uint",
                "minimum": 0,
                "type": [
                  "integer",
                  "null"
                ]
              },
              "path": {
                "type": "string"
              },
              "start_line": {
                "format": "uint",
                "minimum": 0,
                "type": [
                  "integer",
                  "null"
                ]
              }
            },
            "required": [
              "path"
            ],
            "type": "object"
          },
        },
      ]),
    );
  }
}
