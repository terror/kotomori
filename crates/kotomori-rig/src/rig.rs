use {
  super::*,
  ::rig::{
    DynModel,
    completion::{self, CompletionRequest},
    message,
    operation::Completion,
    providers::{
      cohere::Cohere, ollama::Ollama, openai::wire::Dialect,
      registry::ProviderRef,
    },
    streaming::{Item, StreamEvent},
  },
};

#[derive(Clone, Debug)]
pub struct Rig {
  model: DynModel<Completion>,
}

impl Rig {
  #[must_use]
  pub fn build(model: impl Into<DynModel<Completion>>) -> Arc<dyn Provider> {
    Arc::new(Self {
      model: model.into(),
    })
  }

  pub fn cohere(model: &str) -> Result<Arc<dyn Provider>> {
    Ok(Self::build(Cohere::from_env()?.completion(model)))
  }

  fn decode_call_id(id: message::CallId) -> CallId {
    match id {
      message::CallId::Local(id) => CallId::Local(id.to_string()),
      message::CallId::Provider(message::ProviderCallId {
        call_id,
        item_id,
      }) => CallId::Provider { call_id, item_id },
    }
  }

  fn decode_content(
    content: message::AssistantContent,
  ) -> Result<AssistantContent> {
    Ok(match content {
      message::AssistantContent::Image(image) => AssistantContent::Image(
        serde_json::to_value(image)
          .context("failed to decode provider content")?,
      ),
      message::AssistantContent::Reasoning(reasoning) => {
        let issuer = reasoning.issuer().as_str().to_owned();

        let message::Reasoning { content, id } = reasoning
          .open(reasoning.issuer())
          .context("failed to open provider reasoning")?;

        AssistantContent::Reasoning(Reasoning {
          content: content.iter().map(Self::decode_reasoning_content).collect(),
          id: id.clone(),
          issuer,
        })
      }
      message::AssistantContent::Text(message::Text {
        additional_params,
        text,
      }) => AssistantContent::Text(Text {
        additional_params: additional_params
          .map(message::AdditionalParams::into_value),
        text,
      }),
      message::AssistantContent::ToolCall(message::ToolCall {
        additional_params,
        function: message::ToolFunction { arguments, name },
        id,
        signature,
      }) => AssistantContent::ToolCall(ToolRequest {
        additional_params: additional_params.filter(|value| !value.is_null()),
        function: ToolFunction {
          arguments,
          name: ToolName::new(String::from(name))
            .context("failed to decode provider content")?,
        },
        id: Self::decode_call_id(id),
        signature,
      }),
    })
  }

  fn decode_reasoning_content(
    content: &message::ReasoningContent,
  ) -> ReasoningContent {
    match content {
      message::ReasoningContent::Encrypted(content) => {
        ReasoningContent::Encrypted(content.clone())
      }
      message::ReasoningContent::Redacted { data } => {
        ReasoningContent::Redacted { data: data.clone() }
      }
      message::ReasoningContent::Summary(content) => {
        ReasoningContent::Summary(content.clone())
      }
      message::ReasoningContent::Text { signature, text } => {
        ReasoningContent::Text {
          signature: signature.clone(),
          text: text.clone(),
        }
      }
    }
  }

  fn decode_response(
    response: completion::CompletionResponse,
  ) -> Result<ProviderResponse> {
    Ok(ProviderResponse {
      finish_reason: response.finish_reason().map(|reason| match reason {
        completion::FinishReason::ContentFilter => FinishReason::ContentFilter,
        completion::FinishReason::Length => FinishReason::Length,
        completion::FinishReason::Other(reason) => FinishReason::Other(reason),
        completion::FinishReason::Stop => FinishReason::Stop,
        completion::FinishReason::ToolCalls => FinishReason::ToolCalls,
      }),
      message: AgentMessage {
        content: response
          .choice
          .into_iter()
          .map(Self::decode_content)
          .collect::<Result<Vec<_>>>()?,
        id: response.message_id,
      },
      usage: Usage {
        cache_creation_input_tokens: response.usage.cache_creation_input_tokens,
        cached_input_tokens: response.usage.cached_input_tokens,
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
        reasoning_tokens: response.usage.reasoning_tokens,
        tool_use_prompt_tokens: response.usage.tool_use_prompt_tokens,
        total_tokens: response.usage.total_tokens,
      },
    })
  }

  fn encode_call_id(id: &CallId) -> Result<message::CallId> {
    Ok(match id {
      CallId::Local(id) => message::CallId::Local(
        message::LocalCallId::deserialize(
          value::StrDeserializer::<value::Error>::new(id),
        )
        .context("failed to encode tool call identity")?,
      ),
      CallId::Provider { call_id, item_id } => {
        let id = message::ProviderCallId::new(call_id.clone())
          .ok_or(message::EmptyCallId)
          .context("failed to encode tool call identity")?;

        message::CallId::Provider(if let Some(item_id) = item_id {
          id.with_item_id(item_id)
        } else {
          id
        })
      }
    })
  }

  fn encode_content(
    content: &AssistantContent,
  ) -> Result<message::AssistantContent> {
    Ok(match content {
      AssistantContent::Image(image) => message::AssistantContent::Image(
        serde_json::from_value(image.clone())
          .context("failed to encode provider content")?,
      ),
      AssistantContent::Reasoning(Reasoning {
        content,
        id,
        issuer,
      }) => message::AssistantContent::Reasoning(
        message::Reasoning {
          content: content.iter().map(Self::encode_reasoning_content).collect(),
          id: id.clone(),
        }
        .sealed(issuer.clone()),
      ),
      AssistantContent::Text(Text {
        additional_params,
        text,
      }) => message::AssistantContent::Text(message::Text {
        additional_params: message::AdditionalParams::try_from_value(
          additional_params.clone().unwrap_or_default(),
        )
        .map_err(|_| {
          anyhow!("`additional_params` must be a JSON object (or null)")
        })
        .context("failed to encode provider content")?,
        text: text.clone(),
      }),
      AssistantContent::ToolCall(ToolRequest {
        additional_params,
        function: ToolFunction { arguments, name },
        id,
        signature,
      }) => message::AssistantContent::ToolCall(message::ToolCall {
        additional_params: additional_params
          .clone()
          .filter(|value| !value.is_null()),
        function: message::ToolFunction {
          arguments: arguments.clone(),
          name: message::ToolName::new(name.as_str())
            .context("failed to encode provider content")?,
        },
        id: Self::encode_call_id(id)?,
        signature: signature.clone(),
      }),
    })
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
                  Self::encode_call_id(call)?,
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

  fn encode_reasoning_content(
    content: &ReasoningContent,
  ) -> message::ReasoningContent {
    match content {
      ReasoningContent::Encrypted(content) => {
        message::ReasoningContent::Encrypted(content.clone())
      }
      ReasoningContent::Redacted { data } => {
        message::ReasoningContent::Redacted { data: data.clone() }
      }
      ReasoningContent::Summary(content) => {
        message::ReasoningContent::Summary(content.clone())
      }
      ReasoningContent::Text { signature, text } => {
        message::ReasoningContent::Text {
          signature: signature.clone(),
          text: text.clone(),
        }
      }
    }
  }

  pub fn gateway(
    model: &str,
    name: &'static str,
    base_url: &'static str,
    api_key_env: &'static str,
  ) -> Result<Arc<dyn Provider>> {
    Ok(Self::build(
      OpenAIConfig::from_env_with(&Dialect::gateway(
        name,
        base_url,
        api_key_env,
      ))?
      .client()
      .chat(model),
    ))
  }

  pub fn ollama(model: &str) -> Result<Arc<dyn Provider>> {
    Ok(Self::build(Ollama::from_env()?.completion(model)))
  }

  pub fn openai(model: &str) -> Result<Arc<dyn Provider>> {
    Ok(Self::build(OpenAIConfig::from_env()?.client().chat(model)))
  }

  pub fn provider(provider: &str, model: &str) -> Result<Arc<dyn Provider>> {
    Ok(Self::build(
      ProviderRef::parse(&format!("{provider}:{model}"))?.completion_model()?,
    ))
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
          name: tool.name.to_string(),
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
  ) -> BoxFuture<'a, Result<ProviderResponse>> {
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

      Self::decode_response(stream.finish().await?)
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
      content: "foo".into(),
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
            name: tool.name.to_string(),
            parameters: tool.parameters.clone(),
          })
          .collect::<Vec<_>>()
      );
    }

    case(&[]);
    case(&[ToolDefinition {
      description: "bar".into(),
      name: ToolName::new("foo").unwrap(),
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

    for update in [
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
    ] {
      buffer.apply(update);
    }

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
        .map(|content| Rig::encode_content(content).unwrap())
        .collect::<Vec<_>>(),
      expected,
    );
  }
}
