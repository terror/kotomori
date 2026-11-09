use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Mock;

impl Provider for Mock {
  #[allow(clippy::too_many_lines)]
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<ProviderResponse>> {
    Box::pin(async move {
      let content = match request.model.name.as_str() {
        "approval-required-command" => {
          if request.messages.iter().any(Message::has_tool_result) {
            AssistantContent::text("done")
          } else {
            return Ok(ProviderResponse {
              message: vec![
                AssistantContent::text("baz"),
                AssistantContent::ToolCall(ToolRequest::new(
                  CallId::new("foo"),
                  ToolFunction {
                    arguments: serde_json::json!({
                      "command": "echo bar",
                    }),
                    name: ToolName::new("command")?,
                  },
                )),
                AssistantContent::text("qux"),
              ]
              .into(),
              ..Default::default()
            });
          }
        }
        "command-output" if request.messages.len() == 1 => {
          AssistantContent::tool_call(
            "foo",
            ToolName::new("command")?,
            serde_json::json!({
              "command": if cfg!(windows) { "type foo" } else { "cat foo" },
            }),
          )
        }
        model @ ("dual-id-tool-calls"
        | "duplicate-tool-calls"
        | "empty-tool-call"
        | "failed-response"
        | "invalid-local-tool-call")
          if request.messages.len() == 1 =>
        {
          let calls = match model {
            "dual-id-tool-calls" => {
              ["bar", "baz"].map(|item_id| CallId::Provider {
                call_id: "foo".into(),
                item_id: Some(item_id.into()),
              })
            }
            "empty-tool-call" | "failed-response" => {
              [CallId::new("foo"), CallId::new("")]
            }
            "invalid-local-tool-call" => {
              [CallId::new("foo"), CallId::Local("bar".into())]
            }
            _ => [CallId::new("foo"), CallId::new("foo")],
          };

          sink.update(MessageUpdate::Text {
            delta: "bar".into(),
            index: 0,
          })?;

          return Ok(ProviderResponse {
            finish_reason: (model == "failed-response")
              .then(|| FinishReason::Error("foo".into())),
            message: std::iter::once(AssistantContent::text("bar"))
              .chain(calls.into_iter().map(|id| {
                AssistantContent::ToolCall(ToolRequest::new(
                  id,
                  ToolFunction {
                    arguments: serde_json::json!({"command": "echo bar>foo"}),
                    name: ToolName::new("command").unwrap(),
                  },
                ))
              }))
              .collect::<Vec<_>>()
              .into(),
            ..Default::default()
          });
        }
        "empty-reasoning" => AssistantContent::reasoning("mock", ""),
        "encrypted-reasoning" => AssistantContent::Reasoning(Reasoning::new(
          "mock",
          vec![ReasoningContent::Encrypted("bar".into())],
        )),
        "error" if request.messages.len() == 1 => {
          bail!("foo\nbar");
        }
        "escaped-command" if request.messages.len() == 1 => {
          AssistantContent::tool_call(
            "foo",
            ToolName::new("command")?,
            serde_json::json!({
              "command": "echo foo\r\x1b[2J\n? y approve",
            }),
          )
        }
        "failed-command" if request.messages.len() == 1 => {
          AssistantContent::tool_call(
            "foo",
            ToolName::new("command")?,
            serde_json::json!({
              "command": "echo bar&&echo baz>&2&&exit 1",
              "cwd": "foo",
            }),
          )
        }
        "generation-options" => {
          AssistantContent::text(serde_json::to_string(request.generation)?)
        }
        "malformed-tool-arguments" if request.messages.len() == 1 => {
          let content = vec![
            AssistantContent::tool_call(
              "foo",
              ToolName::new("command")?,
              serde_json::json!({"command": "echo bar>foo"}),
            ),
            AssistantContent::ToolCall(ToolRequest::new(
              CallId::new("bar"),
              ToolFunction {
                arguments: serde_json::json!({}),
                name: ToolName::new("command")?,
              },
            )),
          ];

          sink.update_many(content.iter().cloned().enumerate().map(
            |(index, content)| MessageUpdate::Content { index, content },
          ))?;

          return Ok(ProviderResponse {
            message: content.into(),
            ..Default::default()
          });
        }
        "reasoning" => {
          let mut reasoning = Reasoning::new(
            "mock",
            vec![ReasoningContent::Text {
              text: "bar\nbaz".into(),
              signature: Some("foo".into()),
            }],
          );

          reasoning.content.extend([
            ReasoningContent::Summary("qux".into()),
            ReasoningContent::Encrypted("quux".into()),
            ReasoningContent::Redacted {
              data: "quuz".into(),
            },
          ]);

          return Ok(ProviderResponse {
            message: vec![
              AssistantContent::text("corge"),
              AssistantContent::Reasoning(reasoning),
              AssistantContent::text("grault"),
            ]
            .into(),
            ..Default::default()
          });
        }
        "unfinished-reasoning" => {
          sink.update_many([
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
          ])?;

          return pending().await;
        }
        "unknown-tool" if request.messages.len() == 1 => {
          AssistantContent::ToolCall(ToolRequest::new(
            CallId::new("foo"),
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

      Ok(ProviderResponse {
        message: vec![content].into(),
        ..Default::default()
      })
    })
  }
}
