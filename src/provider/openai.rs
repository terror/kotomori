use super::*;

#[derive(Debug, Clone)]
pub(crate) struct OpenAi {
  client: async_openai::Client<OpenAIConfig>,
}

#[derive(Default)]
struct PendingToolCall {
  arguments: String,
  id: Option<String>,
  name: Option<String>,
}

impl OpenAi {
  pub(crate) fn new() -> Self {
    Self::with_config(OpenAIConfig::new())
  }

  pub(crate) fn with_config(config: OpenAIConfig) -> Self {
    Self {
      client: async_openai::Client::with_config(config),
    }
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

fn tool(tool: RegisteredTool) -> ChatCompletionTools {
  ChatCompletionTools::Function(ChatCompletionTool {
    function: FunctionObject {
      description: Some(tool.description.into()),
      name: tool.name.into(),
      parameters: Some(tool.parameters()),
      strict: None,
    },
  })
}

#[async_trait]
impl Provider for OpenAi {
  async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
    let mut stream = self
      .client
      .chat()
      .create_stream((&request).try_into()?)
      .await?;

    let mut tool_calls = BTreeMap::<u32, PendingToolCall>::new();

    while let Some(response) = stream.next().await {
      let response = response?;

      for choice in response.choices {
        if let Some(content) = choice.delta.content
          && !content.is_empty()
        {
          sink.delta(content)?;
        }

        for chunk in choice.delta.tool_calls.unwrap_or_default() {
          let call = tool_calls.entry(chunk.index).or_default();

          if let Some(id) = chunk.id {
            call.id = Some(id);
          }

          if let Some(function) = chunk.function {
            if let Some(name) = function.name {
              call.name = Some(name);
            }

            if let Some(arguments) = function.arguments {
              call.arguments.push_str(&arguments);
            }
          }
        }
      }
    }

    for tool_call in tool_calls.into_values() {
      sink.tool_call(tool_call.finish()?)?;
    }

    Ok(())
  }
}

impl TryFrom<&Request> for CreateChatCompletionRequest {
  type Error = Error;

  fn try_from(request: &Request) -> Result<Self> {
    Ok(
      CreateChatCompletionRequestArgs::default()
        .model(request.model_name())
        .messages(request.messages().map(Into::into).collect::<Vec<_>>())
        .tools(tools().into_iter().map(tool).collect::<Vec<_>>())
        .build()?,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chat_completion_request_includes_tools() {
    let request = CreateChatCompletionRequest::try_from(&Request::new(
      "fake:foo".parse().unwrap(),
      vec![Message::new(Role::User, "bar")],
    ))
    .unwrap();

    assert_eq!(
      request
        .tools
        .unwrap()
        .into_iter()
        .map(|tool| match tool {
          ChatCompletionTools::Function(tool) => tool.function.name,
          ChatCompletionTools::Custom(_) => unreachable!(),
        })
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
  fn pending_tool_call_parses_arguments() {
    assert_eq!(
      PendingToolCall {
        arguments: r#"{"path":"bar"}"#.into(),
        id: Some("foo".into()),
        name: Some("read_file".into()),
      }
      .finish()
      .unwrap(),
      ToolCall::new("foo", "read_file", json!({"path": "bar"})),
    );
  }
}
