use super::*;

#[derive(Debug)]
pub(crate) struct Ollama {
  client: crate::ollama::Ollama,
}

impl Ollama {
  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      client: crate::ollama::Ollama::try_new(
        env::var("OLLAMA_HOST")
          .unwrap_or_else(|_| "http://localhost:11434".into()),
      )?,
    })
  }
}

#[async_trait]
impl Provider for Ollama {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    let mut stream = self
      .client
      .send_chat_messages_stream(crate::ollama::ChatMessageRequest::try_from(
        &request,
      )?)
      .await?;

    let mut next_tool_call: usize = 0;

    while let Some(response) = stream.next().await {
      let response =
        response.map_err(|()| anyhow!("failed to read Ollama stream"))?;

      let message = response.message;

      if let Some(thinking) =
        message.thinking.filter(|thinking| !thinking.is_empty())
      {
        sink.reasoning_delta(thinking)?;
      }

      if !message.content.is_empty() {
        sink.delta(message.content)?;
      }

      for tool_call in message.tool_calls {
        let index = next_tool_call;

        next_tool_call = next_tool_call.saturating_add(1);

        let arguments = match tool_call.function.arguments {
          Value::String(arguments) => {
            serde_json::from_str(&arguments).unwrap_or(Value::String(arguments))
          }
          arguments => arguments,
        };

        sink.tool_call(RawToolCall::new(
          format!("ollama-tool-{index}"),
          tool_call.function.name,
          arguments,
        ))?;
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chat_request_builds_messages_and_tools() {
    let tool = Tool::new::<ReadFileTool>();
    let parameters = tool.parameters.clone();

    let request =
      crate::ollama::ChatMessageRequest::try_from(&Request::with_system(
        "ollama:foo".parse().unwrap(),
        vec![
          Message::tool_use_with_reasoning(
            "foo",
            "read_file",
            json!({"path": "bar"}),
            Some("baz".into()),
          ),
          Message::tool_result("foo", "qux", false),
        ],
        "quux",
        ToolRegistry::new(vec![tool]),
      ))
      .unwrap();

    assert_eq!(
      serde_json::to_value(&request).unwrap()["messages"],
      json!([
        {
          "role": "system",
          "content": "quux",
          "tool_calls": [],
          "thinking": null,
        },
        {
          "role": "assistant",
          "content": "",
          "thinking": "baz",
          "tool_calls": [
            {
              "function": {
                "name": "read_file",
                "arguments": {"path": "bar"},
              },
            },
          ],
        },
        {
          "role": "tool",
          "content": "qux",
          "tool_calls": [],
          "thinking": null,
        },
      ]),
    );

    assert_eq!(serde_json::to_value(&request).unwrap()["model"], "foo");

    assert!(
      serde_json::to_value(&request)
        .unwrap()
        .get("think")
        .is_none()
    );

    assert_eq!(
      serde_json::to_value(&request).unwrap()["tools"][0]["function"]["parameters"],
      parameters,
    );
  }
}
