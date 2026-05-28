use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Mock;

#[async_trait]
impl Provider for Mock {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    match request.model_name() {
      "command" => {
        if request.has_tool_result() {
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
      }
      model => {
        let input = request
          .last_user_message()
          .and_then(Message::content)
          .unwrap_or_default();

        let response = format!("queued for mock:{model}: {input}");

        for c in response.chars() {
          sink.delta(c.to_string())?;
          sleep(Duration::from_millis(20)).await;
        }
      }
    }

    Ok(())
  }
}
