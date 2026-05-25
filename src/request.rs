use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Request {
  messages: Vec<Message>,
  model: Model,
}

impl Request {
  pub(crate) fn last_user_message(&self) -> Option<&Message> {
    self
      .messages()
      .rev()
      .find(|message| message.role == Role::User)
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

  pub(crate) fn new(model: Model, messages: Vec<Message>) -> Self {
    Self { messages, model }
  }
}

fn anthropic_tool(tool: &RegisteredTool) -> types::Tool {
  let Value::Object(schema) = tool.parameters() else {
    unreachable!()
  };

  let properties = schema
    .get("properties")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();

  let required = schema
    .get("required")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::to_string)
    .collect();

  let additional = schema
    .into_iter()
    .filter(|(key, _)| {
      key != "properties" && key != "required" && key != "type"
    })
    .collect();

  types::Tool {
    description: tool.description.into(),
    input_schema: types::ToolInputSchema {
      additional,
      properties,
      required,
      schema_type: "object".into(),
    },
    name: tool.name.into(),
  }
}

fn openai_tool(tool: &RegisteredTool) -> ChatCompletionTools {
  ChatCompletionTools::Function(ChatCompletionTool {
    function: FunctionObject {
      description: Some(tool.description.into()),
      name: tool.name.into(),
      parameters: Some(tool.parameters()),
      strict: None,
    },
  })
}

impl From<&Request> for types::MessageCreateParams {
  fn from(request: &Request) -> Self {
    request
      .messages()
      .map(types::MessageParam::from)
      .fold(
        types::MessageCreateBuilder::new(
          request.model_name(),
          env::var("ANTHROPIC_MAX_TOKENS")
            .ok()
            .and_then(|max_tokens| max_tokens.parse::<u32>().ok())
            .unwrap_or(4096),
        ),
        |builder, message| builder.message(message.role, message.content),
      )
      .tools(
        inventory::iter::<RegisteredTool>
          .into_iter()
          .map(anthropic_tool)
          .collect::<Vec<_>>(),
      )
      .build()
  }
}

impl TryFrom<&Request> for CreateChatCompletionRequest {
  type Error = Error;

  fn try_from(request: &Request) -> Result<Self> {
    Ok(
      CreateChatCompletionRequestArgs::default()
        .model(request.model_name())
        .messages(request.messages().map(Into::into).collect::<Vec<_>>())
        .tools(
          inventory::iter::<RegisteredTool>
            .into_iter()
            .map(openai_tool)
            .collect::<Vec<_>>(),
        )
        .build()?,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chat_messages() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
      ],
    );

    assert_eq!(request.model_name(), "foo");

    assert_eq!(
      request
        .messages()
        .map(|message| (message.role, message.content.as_str()))
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
    );

    assert_eq!(request.last_user_message().unwrap().content, "baz");
  }
}
