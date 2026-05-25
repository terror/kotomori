use super::*;

#[derive(Debug, Clone)]
pub(crate) struct OpenAi {
  client: async_openai::Client<OpenAIConfig>,
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
      for choice in response?.choices {
        if let Some(content) =
          choice.delta.content.filter(|content| !content.is_empty())
        {
          sink.delta(content)?;
        }

        for chunk in choice.delta.tool_calls.into_iter().flatten() {
          tool_calls.entry(chunk.index).or_default().append(chunk);
        }
      }
    }

    for tool_call in tool_calls.into_values() {
      sink.tool_call(tool_call.finish()?)?;
    }

    Ok(())
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

    let mut tools = request
      .tools
      .unwrap()
      .into_iter()
      .map(|tool| match tool {
        ChatCompletionTools::Function(tool) => tool.function.name,
        ChatCompletionTools::Custom(_) => unreachable!(),
      })
      .collect::<Vec<_>>();

    tools.sort();

    assert_eq!(
      tools,
      vec![
        "apply_patch",
        "command",
        "list_files",
        "read_file",
        "search_files",
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
      RawToolCall::new("foo", "read_file", json!({"path": "bar"})),
    );
  }
}
