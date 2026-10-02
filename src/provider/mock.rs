use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Mock;

#[async_trait]
impl Provider for Mock {
  async fn stream(
    &self,
    request: Request,
    sink: &ProviderSink,
  ) -> Result<AgentMessage> {
    let content = match request.model.name.as_str() {
      "approval-required-command" => {
        let has_tool_result =
          request.messages.iter().any(|message| match message {
            Message::Agent(_) => false,
            Message::User(content) => content.iter().any(|content| {
              matches!(content, UserMessageContent::ToolResult { .. })
            }),
          });

        if has_tool_result {
          AssistantContent::text("done")
        } else {
          AssistantContent::ToolCall(::rig::message::ToolCall::from_wire(
            "foo",
            ToolFunction {
              arguments: serde_json::json!({
                "command": "echo bar",
              }),
              name: ToolName::new("command")?,
            },
          ))
        }
      }
      "error" if request.messages.len() == 1 => {
        bail!("mock provider error");
      }
      "malformed-tool-arguments" if request.messages.len() == 1 => {
        AssistantContent::ToolCall(::rig::message::ToolCall::from_wire(
          "foo",
          ToolFunction {
            arguments: serde_json::json!({}),
            name: ToolName::new("command")?,
          },
        ))
      }
      "unknown-tool" if request.messages.len() == 1 => {
        AssistantContent::ToolCall(::rig::message::ToolCall::from_wire(
          "foo",
          ToolFunction {
            arguments: serde_json::json!({}),
            name: ToolName::new("unknown")?,
          },
        ))
      }
      model => AssistantContent::text(format!(
        "queued for mock:{model}: {}",
        request.last_user_text().unwrap_or_default()
      )),
    };

    if let AssistantContent::Text(text) = &content {
      if request.model.name == "slow-streaming" {
        for c in text.text.chars() {
          sink.update(MessageUpdate::Text {
            delta: c.to_string(),
            index: 0,
          })?;

          sleep(Duration::from_millis(20)).await;
        }
      } else {
        sink.update(MessageUpdate::Text {
          delta: text.text.clone(),
          index: 0,
        })?;
      }
    }

    Ok(vec![content].into())
  }
}
