use {
  super::*,
  ::rig::{
    client::CompletionClient, completion::CompletionModel,
    streaming::StreamedAssistantContent,
  },
};

#[derive(Clone)]
pub(super) struct Rig<M> {
  model: M,
  provider: String,
}

impl<M> Rig<M>
where
  M: CompletionModel + 'static,
{
  pub(super) fn build(
    client: &impl CompletionClient<CompletionModel = M>,
    model: &Model,
  ) -> Arc<dyn Provider> {
    Arc::new(Self {
      model: client.completion_model(&model.name),
      provider: model.provider.clone(),
    })
  }
}

impl<M> Debug for Rig<M> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Rig")
      .field("provider", &self.provider)
      .finish_non_exhaustive()
  }
}

#[async_trait]
impl<M> Provider for Rig<M>
where
  M: CompletionModel + 'static,
{
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    let request = CompletionRequest::from(&request);

    let mut stream = self.model.stream(request).await?;

    loop {
      let chunk = stream.next().await;

      if let Some(id) = stream.message_id.take() {
        sink.message_id(id)?;
      }

      let Some(chunk) = chunk else {
        break;
      };

      match chunk? {
        StreamedAssistantContent::Text(text) if !text.text.is_empty() => {
          sink.delta(text.text)?;
        }
        StreamedAssistantContent::Reasoning(reasoning)
          if self.provider == "gemini" =>
        {
          sink.reasoning_append(reasoning)?;
        }
        StreamedAssistantContent::Reasoning(reasoning) => {
          sink.reasoning(reasoning)?;
        }
        StreamedAssistantContent::ReasoningDelta { id, reasoning }
          if !reasoning.is_empty() =>
        {
          sink.reasoning_delta(id, reasoning)?;
        }
        StreamedAssistantContent::ToolCall {
          internal_call_id,
          mut tool_call,
        } => {
          if tool_call.id.is_empty() {
            tool_call.id = internal_call_id;
          }

          sink.tool_call(tool_call);
        }
        StreamedAssistantContent::Final(_)
        | StreamedAssistantContent::ReasoningDelta { .. }
        | StreamedAssistantContent::Text(_)
        | StreamedAssistantContent::ToolCallDelta { .. } => {}
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    ::rig::{
      completion::{CompletionError, CompletionResponse},
      streaming::{
        RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse,
      },
    },
    futures_util::stream,
    serde_json::json,
  };

  #[derive(Clone)]
  struct TestModel(Vec<RawStreamingChoice<()>>);

  impl CompletionModel for TestModel {
    type Client = Self;
    type Response = ();
    type StreamingResponse = ();

    async fn completion(
      &self,
      _: CompletionRequest,
    ) -> Result<CompletionResponse<()>, CompletionError> {
      unreachable!();
    }

    fn make(client: &Self, _: impl Into<String>) -> Self {
      client.clone()
    }

    fn stream(
      &self,
      _: CompletionRequest,
    ) -> impl Future<Output = Result<StreamingCompletionResponse<()>, CompletionError>>
    {
      future::ready(Ok(StreamingCompletionResponse::stream(Box::pin(
        stream::iter(self.0.clone().into_iter().map(Ok)),
      ))))
    }
  }

  async fn collect(
    provider: &str,
    output: Vec<RawStreamingChoice<()>>,
  ) -> (AgentMessage, Vec<Event>) {
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();

    let mut sink = ProviderSink {
      event_sender,
      ..ProviderSink::default()
    };

    Rig {
      model: TestModel(output),
      provider: provider.into(),
    }
    .stream(
      Request {
        messages: Vec::new(),
        model: "mock:foo".parse().unwrap(),
        system: None,
      },
      &mut sink,
    )
    .await
    .unwrap();

    let message = sink.finish();

    let mut events = Vec::new();

    while let Some(event) = event_receiver.recv().await {
      events.push(event);
    }

    (message, events)
  }

  #[tokio::test]
  async fn normalizes_signed_reasoning() {
    for (provider, text) in [("anthropic", "foobar"), ("gemini", "bar")] {
      let reasoning = Reasoning::new_with_signature(text, Some("baz".into()));

      let (message, events) = collect(
        provider,
        vec![
          RawStreamingChoice::ReasoningDelta {
            id: None,
            reasoning: "foo".into(),
          },
          RawStreamingChoice::Reasoning {
            id: None,
            content: ReasoningContent::Text {
              text: text.into(),
              signature: Some("baz".into()),
            },
          },
        ],
      )
      .await;

      assert_eq!(
        message,
        AgentMessage::from(vec![AssistantContent::Reasoning(
          Reasoning::new_with_signature("foobar", Some("baz".into()))
        )])
      );

      assert_eq!(
        events,
        vec![
          Event::Agent {
            event: AgentEvent::Update(MessageUpdate::ReasoningDelta {
              delta: "foo".into(),
              id: None,
            }),
            run_id: 0,
          },
          Event::Agent {
            event: AgentEvent::Update(if provider == "gemini" {
              MessageUpdate::ReasoningAppend(reasoning)
            } else {
              MessageUpdate::Reasoning(reasoning)
            }),
            run_id: 0,
          },
        ],
      );
    }
  }

  #[tokio::test]
  async fn preserves_reasoning_parts() {
    let (message, _) = collect(
      "openai",
      vec![
        RawStreamingChoice::ReasoningDelta {
          id: None,
          reasoning: "foo".into(),
        },
        RawStreamingChoice::Reasoning {
          id: Some("bar".into()),
          content: ReasoningContent::Summary("foo".into()),
        },
        RawStreamingChoice::Reasoning {
          id: Some("bar".into()),
          content: ReasoningContent::Summary("foo".into()),
        },
        RawStreamingChoice::Reasoning {
          id: Some("bar".into()),
          content: ReasoningContent::Encrypted("baz".into()),
        },
      ],
    )
    .await;

    let mut reasoning = Reasoning::summaries(vec!["foo".into(), "foo".into()])
      .with_id("bar".into());

    reasoning
      .content
      .push(ReasoningContent::Encrypted("baz".into()));

    assert_eq!(
      message,
      AgentMessage::from(vec![AssistantContent::Reasoning(reasoning)])
    );
  }

  #[tokio::test]
  async fn preserves_tool_call_metadata() {
    for (id, expected) in [("foo", "foo"), ("", "bar")] {
      let call = RawStreamingToolCall::new(
        id.into(),
        "command".into(),
        json!({"command": "baz", "cwd": null}),
      )
      .with_internal_call_id("bar".into())
      .with_call_id("qux".into())
      .with_signature(Some("quux".into()))
      .with_additional_params(Some(json!({"foo": "bar"})));

      let (message, events) = collect(
        "foo",
        vec![
          RawStreamingChoice::ToolCall(call),
          RawStreamingChoice::MessageId("quuz".into()),
        ],
      )
      .await;

      assert_eq!(
        RigMessage::from(&Message::Agent(message)),
        RigMessage::Assistant {
          content: OneOrMany::one(AssistantContent::ToolCall(
            ::rig::message::ToolCall {
              additional_params: Some(json!({"foo": "bar"})),
              call_id: Some("qux".into()),
              function: ToolFunction {
                arguments: json!({"command": "baz", "cwd": null}),
                name: "command".into()
              },
              id: expected.into(),
              signature: Some("quux".into()),
            }
          )),
          id: Some("quuz".into()),
        }
      );

      assert_eq!(
        events,
        vec![Event::Agent {
          event: AgentEvent::Update(MessageUpdate::MessageId("quuz".into())),
          run_id: 0,
        }]
      );
    }
  }
}
