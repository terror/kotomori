use super::*;

pub(crate) struct Anthropic {
  client: anthropic_sdk::Anthropic,
}

#[derive(Default)]
struct PendingToolCall {
  arguments: String,
  id: Option<String>,
  name: Option<String>,
}

impl Anthropic {
  pub(crate) fn new() -> Result<Self> {
    let base_url = env::var("ANTHROPIC_BASE_URL")
      .unwrap_or_else(|_| "https://api.anthropic.com".into());

    let api_key = env::var("ANTHROPIC_API_KEY")
      .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN"))
      .unwrap_or_default();

    Ok(Self {
      client: anthropic_sdk::Anthropic::with_config(
        anthropic_sdk::ClientConfig::new(api_key)
          .with_base_url(base_url)
          .with_auth_method(AuthMethod::Anthropic),
      )?,
    })
  }
}

impl PendingToolCall {
  fn finish(self) -> Result<ToolCall> {
    let id = self.id.context("missing tool call id")?;
    let name = self.name.context("missing tool call name")?;
    let arguments = if self.arguments.trim().is_empty() {
      "{}"
    } else {
      &self.arguments
    };

    ToolCall::from_arguments_string(id, name, arguments)
  }
}

impl Debug for Anthropic {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Anthropic").finish_non_exhaustive()
  }
}

fn tool(tool: RegisteredTool) -> types::Tool {
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

fn tool_arguments(input: Value) -> String {
  match input {
    Value::Null => String::new(),
    Value::Object(object) if object.is_empty() => String::new(),
    input => input.to_string(),
  }
}

#[async_trait]
impl Provider for Anthropic {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
    let mut stream = self
      .client
      .messages()
      .create_stream((&request).into())
      .await?;

    let mut tool_calls = BTreeMap::<usize, PendingToolCall>::new();

    while let Some(event) = stream.next().await {
      let event = event?;

      match event {
        types::MessageStreamEvent::ContentBlockStart {
          content_block: types::ContentBlock::ToolUse { id, input, name },
          index,
        } => {
          tool_calls.insert(
            index,
            PendingToolCall {
              arguments: tool_arguments(input),
              id: Some(id),
              name: Some(name),
            },
          );
        }
        types::MessageStreamEvent::ContentBlockDelta {
          delta: types::ContentBlockDelta::TextDelta { text },
          ..
        } if !text.is_empty() => sink.delta(text)?,
        types::MessageStreamEvent::ContentBlockDelta {
          delta: types::ContentBlockDelta::InputJsonDelta { partial_json },
          index,
        } => {
          tool_calls
            .entry(index)
            .or_default()
            .arguments
            .push_str(&partial_json);
        }
        types::MessageStreamEvent::ContentBlockStop { index } => {
          if let Some(tool_call) = tool_calls.remove(&index) {
            sink.tool_call(tool_call.finish()?)?;
          }
        }
        _ => {}
      }
    }

    Ok(())
  }
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
      .tools(tools().into_iter().map(tool).collect::<Vec<_>>())
      .build()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn message_create_params_include_tools() {
    let request = types::MessageCreateParams::from(&Request::new(
      "fake:foo".parse().unwrap(),
      vec![Message::new(Role::User, "bar")],
    ));

    assert_eq!(
      request
        .tools
        .unwrap()
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>(),
      vec![
        "list_files",
        "search_files",
        "read_file",
        "command",
        "apply_patch"
      ],
    );
  }

  #[test]
  fn pending_tool_call_defaults_to_empty_arguments() {
    assert_eq!(
      PendingToolCall {
        arguments: String::new(),
        id: Some("foo".into()),
        name: Some("list_files".into()),
      }
      .finish()
      .unwrap(),
      ToolCall::new("foo", "list_files", json!({})),
    );
  }
}
