use super::*;

#[derive(Clone, Copy, Debug)]
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
          sink.update(MessageUpdate::Text {
            delta: "done".into(),
            index: 0,
          })?;
        } else {
          sink.update(MessageUpdate::Content {
            content: AssistantContent::ToolCall(
              ::rig::message::ToolCall::from_wire(
                "foo",
                ToolFunction {
                  arguments: serde_json::json!({
                    "command": "echo bar",
                  }),
                  name: ToolName::new("command")?,
                },
              ),
            ),
            index: 0,
          })?;
        }
      }
      "error" if request.messages.len() == 1 => {
        bail!("mock provider error");
      }
      "malformed-tool-arguments" if request.messages.len() == 1 => {
        sink.update(MessageUpdate::Content {
          content: AssistantContent::ToolCall(
            ::rig::message::ToolCall::from_wire(
              "foo",
              ToolFunction {
                arguments: serde_json::json!({}),
                name: ToolName::new("command")?,
              },
            ),
          ),
          index: 0,
        })?;
      }
      "unknown-tool" if request.messages.len() == 1 => {
        sink.update(MessageUpdate::Content {
          content: AssistantContent::ToolCall(
            ::rig::message::ToolCall::from_wire(
              "foo",
              ToolFunction {
                arguments: serde_json::json!({}),
                name: ToolName::new("unknown")?,
              },
            ),
          ),
          index: 0,
        })?;
      }
      model => {
        let input = request.last_user_text().unwrap_or_default();

        let response = format!("queued for mock:{model}: {input}");

        if model == "slow-streaming" {
          for c in response.chars() {
            sink.update(MessageUpdate::Text {
              delta: c.to_string(),
              index: 0,
            })?;
            sleep(Duration::from_millis(20)).await;
          }
        } else {
          sink.update(MessageUpdate::Text {
            delta: response,
            index: 0,
          })?;
        }
      }
    }

    Ok(())
  }
}
