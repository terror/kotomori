use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Fake;

impl Fake {
  fn has_tool_result(request: &Request) -> bool {
    request.messages().any(|message| {
      matches!(
        message,
        Message::User(content) if content
          .iter()
          .any(|content| matches!(content, UserMessageContent::ToolResult { .. }))
      )
    })
  }

  fn stream_command(request: &Request, sink: &mut ProviderSink) -> Result {
    if Self::has_tool_result(request) {
      sink.delta("done")?;
    } else {
      sink.tool_call(RawToolCall::new(
        "foo",
        "command",
        serde_json::json!({
          "arguments": ["bar"],
          "cwd": null,
          "program": "echo",
        }),
      ))?;
    }

    Ok(())
  }
}

#[async_trait]
impl Provider for Fake {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    if request.model_name() == "command" {
      return Self::stream_command(&request, sink);
    }

    let input = request
      .last_user_message()
      .and_then(Message::content)
      .unwrap_or_default();

    let response = format!("queued for {}: {input}", request.model());

    for c in response.chars() {
      sink.delta(c.to_string())?;
      sleep(Duration::from_millis(20)).await;
    }

    Ok(())
  }
}
