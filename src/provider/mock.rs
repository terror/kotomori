use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Mock;

#[async_trait]
impl Provider for Mock {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    match request.model.name.as_str() {
      "approval-required-command" => {
        let has_tool_result =
          request.messages.iter().any(|message| match message {
            Message::Agent(_) => false,
            Message::User(content) => content.iter().any(|content| {
              matches!(content, UserMessageContent::ToolResult { .. })
            }),
          });

        if has_tool_result {
          sink.delta("done")?;
        } else {
          sink.tool_call(RawToolCall {
            arguments: serde_json::json!({
              "arguments": ["bar"],
              "cwd": null,
              "program": "echo",
            }),
            id: "foo".into(),
            name: "command".into(),
          })?;
        }
      }
      model => {
        let input = request
          .last_user_message()
          .and_then(Message::content)
          .unwrap_or_default();

        let response = format!("queued for mock:{model}: {input}");

        if model == "slow-streaming" {
          for c in response.chars() {
            sink.delta(c.to_string())?;
            sleep(Duration::from_millis(20)).await;
          }
        } else {
          sink.delta(response)?;
        }
      }
    }

    Ok(())
  }
}
