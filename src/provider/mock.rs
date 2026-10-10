use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Mock;

impl Mock {
  async fn unfinished_reasoning(
    sink: &ProviderSink,
  ) -> Result<AssistantContent> {
    for update in [
      MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      },
      MessageUpdate::ReasoningDelta {
        delta: "baz\nqux".into(),
        index: 1,
      },
      MessageUpdate::Text {
        delta: "quux".into(),
        index: 2,
      },
      MessageUpdate::ReasoningDelta {
        delta: "quuz".into(),
        index: 3,
      },
      MessageUpdate::Text {
        delta: "corge\ngrault".into(),
        index: 4,
      },
    ] {
      sink.update(update)?;
    }

    pending().await
  }
}

impl Provider for Mock {
  fn stream<'a>(
    &'a self,
    request: Request,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<AgentMessage>> {
    Box::pin(async move {
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
        "empty-reasoning" => AssistantContent::reasoning("mock", ""),
        "encrypted-reasoning" => AssistantContent::Reasoning(
          Reasoning::encrypted("bar").sealed("mock"),
        ),
        "error" if request.messages.len() == 1 => {
          bail!("foo\nbar");
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
        "reasoning" => {
          let mut reasoning =
            Reasoning::new_with_signature("bar\nbaz", Some("foo".into()));

          reasoning.content.extend([
            ReasoningContent::Summary("qux".into()),
            ReasoningContent::Encrypted("quux".into()),
            ReasoningContent::Redacted {
              data: "quuz".into(),
            },
          ]);

          return Ok(
            vec![
              AssistantContent::text("corge"),
              AssistantContent::Reasoning(reasoning.sealed("mock")),
              AssistantContent::text("grault"),
            ]
            .into(),
          );
        }
        "unfinished-reasoning" => Self::unfinished_reasoning(sink).await?,
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
    })
  }
}
