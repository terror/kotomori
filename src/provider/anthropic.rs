use super::*;

pub(crate) struct Anthropic {
  client: anthropic_sdk::Anthropic,
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

impl Debug for Anthropic {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Anthropic").finish_non_exhaustive()
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
          tool_calls.insert(index, PendingToolCall::new(id, name, input));
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
            .append_arguments(Some(partial_json));
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn message_create_params_include_tools() {
    let request = types::MessageCreateParams::from(&Request::new(
      "fake:foo".parse().unwrap(),
      vec![Message::new(Role::User, "bar")],
    ));

    let mut tools = request
      .tools
      .unwrap()
      .into_iter()
      .map(|tool| tool.name)
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
  fn pending_tool_call_defaults_to_empty_arguments() {
    assert_eq!(
      PendingToolCall {
        arguments: String::new(),
        id: Some("foo".into()),
        name: Some("list_files".into()),
      }
      .finish()
      .unwrap(),
      RawToolCall::new("foo", "list_files", json!({})),
    );
  }
}
