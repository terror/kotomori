use {
  super::*,
  ::rig::{
    DynModel,
    completion::{self, CompletionRequest},
    message,
    operation::Completion,
    streaming::{Item, StreamEvent},
  },
};

#[derive(Clone, Debug)]
pub(super) struct Rig {
  model: DynModel<Completion>,
}

impl Rig {
  pub(super) fn build(
    model: impl Into<DynModel<Completion>>,
  ) -> Arc<dyn Provider> {
    Arc::new(Self {
      model: model.into(),
    })
  }

  fn decode_content(
    content: message::AssistantContent,
  ) -> Result<AssistantContent> {
    serde_json::from_value(serde_json::to_value(content)?)
      .context("failed to decode provider content")
  }

  fn encode_content(
    content: &AssistantContent,
  ) -> Result<message::AssistantContent> {
    serde_json::from_value(serde_json::to_value(content)?)
      .context("failed to encode provider content")
  }

  fn encode_message(message: &Message) -> Result<message::Message> {
    Ok(match message {
      Message::Agent(message) => message::Message::Assistant {
        content: message
          .content
          .iter()
          .map(Self::encode_content)
          .collect::<Result<Vec<_>>>()?,
        id: message.id.clone(),
      },
      Message::User(content) => message::Message::User {
        content: content
          .iter()
          .map(|content| {
            Ok(match content {
              UserMessageContent::Text(text) => {
                message::UserContent::text(text.clone())
              }
              UserMessageContent::ToolResult { call, name, result } => {
                message::UserContent::tool_result(
                  serde_json::from_value(serde_json::to_value(call)?)
                    .context("failed to encode tool call identity")?,
                  message::ToolName::new(name.as_str())?,
                  vec![message::ToolResultContent::text(
                    result.message_content(),
                  )],
                )
              }
            })
          })
          .collect::<Result<Vec<_>>>()?,
      },
    })
  }

  fn request(request: &Request<'_>) -> Result<CompletionRequest> {
    let chat_history = request
      .system
      .map(message::Message::system)
      .into_iter()
      .map(Ok)
      .chain(request.messages.iter().map(Self::encode_message))
      .collect::<Result<Vec<_>>>()?;

    Ok(CompletionRequest {
      additional_params: None,
      chat_history: if chat_history.is_empty() {
        vec![message::Message::user("")]
      } else {
        chat_history
      },
      documents: Vec::new(),
      max_tokens: None,
      model: Some(request.model.name.clone()),
      output_schema: None,
      record_telemetry_content: false,
      temperature: None,
      tool_choice: None,
      tools: request
        .tools
        .iter()
        .map(|tool| completion::ToolDefinition {
          description: tool.description.clone(),
          name: tool.name.clone(),
          parameters: tool.parameters.clone(),
        })
        .collect(),
    })
  }
}

impl Provider for Rig {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<AgentMessage>> {
    Box::pin(async move {
      let mut stream = self.model.stream(Self::request(&request)?)?;

      let mut message_id = None;

      while let Some(item) = stream.next().await {
        let id = stream.message_id();

        if id != message_id {
          if let Some(id) = &id {
            sink.update(MessageUpdate::MessageId(id.clone()))?;
          }

          message_id = id;
        }

        let Item::Event(event) = item? else {
          continue;
        };

        sink.update(match event {
          StreamEvent::Text { part, text } => MessageUpdate::Text {
            delta: text,
            index: part.index(),
          },
          StreamEvent::Reasoning { part, text } => {
            MessageUpdate::ReasoningDelta {
              delta: text,
              index: part.index(),
            }
          }
          StreamEvent::End { part, content } => MessageUpdate::Content {
            content: Self::decode_content(content)?,
            index: part.index(),
          },
          StreamEvent::Start { .. } | StreamEvent::Arguments { .. } => continue,
        })?;
      }

      let response = stream.finish().await?;

      let message = AgentMessage {
        content: response
          .choice
          .into_iter()
          .map(Self::decode_content)
          .collect::<Result<Vec<_>>>()?,
        id: response.message_id,
      };

      sink.update(MessageUpdate::Complete(message.clone()))?;

      Ok(message)
    })
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn completion_request_uses_blank_user_message_for_empty_history() {
    let request = Rig::request(&Request {
      messages: &[],
      model: &Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: None,
      tools: &[],
    })
    .unwrap();

    assert_eq!(
      request.chat_history.iter().collect::<Vec<_>>(),
      vec![&message::Message::user("")],
    );
  }

  #[test]
  fn completion_request_uses_system_context_and_model() {
    let result = ToolResult {
      content: Some("foo".into()),
      outcome: ToolOutcome::Success,
      ..Default::default()
    };

    let request = Rig::request(&Request {
      messages: &[
        Message::User(vec![UserMessageContent::Text("bar".into())]),
        Message::Agent(vec![AssistantContent::text("qux")].into()),
        Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::Provider {
            call_id: "bar".into(),
            item_id: Some("foo".into()),
          },
          name: ToolName::new("command").unwrap(),
          result: result.clone(),
        }]),
      ],
      model: &Model {
        name: "foo".into(),
        provider: "mock".into(),
      },
      system: Some("baz"),
      tools: &[],
    })
    .unwrap();

    assert_eq!(request.model.as_deref(), Some("foo"));

    assert_eq!(
      request.chat_history.iter().collect::<Vec<_>>(),
      vec![
        &message::Message::system("baz"),
        &message::Message::user("bar"),
        &message::Message::assistant("qux"),
        &message::Message::tool_result(
          message::CallId::from_dual_wire("foo", "bar"),
          message::ToolName::new("command").unwrap(),
          result.message_content(),
        ),
      ],
    );
  }

  #[test]
  fn completion_request_uses_tools() {
    #[track_caller]
    fn case(tools: &[ToolDefinition]) {
      let request = Rig::request(&Request {
        messages: &[],
        model: &Model {
          name: "foo".into(),
          provider: "mock".into(),
        },
        system: None,
        tools,
      })
      .unwrap();

      assert_eq!(
        request.tools,
        tools
          .iter()
          .map(|tool| completion::ToolDefinition {
            description: tool.description.clone(),
            name: tool.name.clone(),
            parameters: tool.parameters.clone(),
          })
          .collect::<Vec<_>>()
      );
    }

    case(&[]);
    case(&[ToolDefinition {
      description: "bar".into(),
      name: "foo".into(),
      parameters: json!({
        "type": "object",
        "properties": {"baz": {"type": "string"}},
        "required": ["baz"],
      }),
    }]);
  }

  #[test]
  fn interleaved_parts_preserve_final_content() {
    let mut buffer = MessageBuffer::default();

    let reasoning = message::AssistantContent::Reasoning(
      message::Reasoning {
        content: vec![
          message::ReasoningContent::Text {
            signature: Some("bar".into()),
            text: "fooé".into(),
          },
          message::ReasoningContent::Summary("baz".into()),
          message::ReasoningContent::Encrypted("qux".into()),
          message::ReasoningContent::Redacted {
            data: "quux".into(),
          },
        ],
        id: Some("quuz".into()),
      }
      .sealed("foo"),
    );

    let tool = message::AssistantContent::ToolCall(
      message::ToolCall::from_dual_wire(
        "foo",
        "bar",
        message::ToolFunction {
          arguments: json!({"command": "baz"}),
          name: message::ToolName::new("command").unwrap(),
        },
      )
      .with_signature(Some("qux".into()))
      .with_additional_params(Some(json!({"foo": "bar"}))),
    );

    let text = message::AssistantContent::Text(message::Text {
      additional_params: message::AdditionalParams::from_entries([(
        "foo",
        json!("bar"),
      )]),
      text: "bazqux".into(),
    });

    let expected = vec![
      reasoning.clone(),
      tool.clone(),
      text.clone(),
      message::AssistantContent::text("quux"),
    ];

    let reasoning = Rig::decode_content(reasoning).unwrap();
    let tool = Rig::decode_content(tool).unwrap();
    let text = Rig::decode_content(text).unwrap();

    let update = |index, delta: &str| MessageUpdate::Text {
      delta: delta.into(),
      index,
    };

    buffer.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        index: 0,
      },
      update(2, "baz"),
      update(3, "quux"),
      MessageUpdate::ReasoningDelta {
        delta: "é".into(),
        index: 0,
      },
      update(2, "qux"),
    ]);

    assert_eq!(
      buffer.preview().cloned().collect::<Vec<_>>(),
      vec![
        MessageDraft::Reasoning("fooé".into()),
        MessageDraft::Content(AssistantContent::text("bazqux")),
        MessageDraft::Content(AssistantContent::text("quux")),
      ]
    );

    for (index, content) in [
      (2, text.clone()),
      (1, tool.clone()),
      (0, reasoning.clone()),
      (0, reasoning.clone()),
    ] {
      buffer.apply(MessageUpdate::Content { index, content });
    }

    buffer.apply(MessageUpdate::MessageId("foo".into()));

    let message = AgentMessage {
      content: vec![reasoning, tool, text, AssistantContent::text("quux")],
      id: Some("foo".into()),
    };

    assert!(!buffer.has_pending_reasoning());

    assert_eq!(buffer.message(), message);
    assert_eq!(buffer.finish(), message);

    let legacy = json!({"content": expected, "id": "foo"});

    assert_eq!(serde_json::to_value(&message).unwrap(), legacy);

    let message = serde_json::from_value::<AgentMessage>(legacy).unwrap();

    assert_eq!(
      message
        .content
        .iter()
        .map(Rig::encode_content)
        .collect::<Result<Vec<_>>>()
        .unwrap(),
      expected,
    );
  }
}
