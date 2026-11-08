use super::*;

#[derive(Debug)]
pub struct Agent {
  config: AgentConfig,
  provider: Arc<dyn Provider>,
  system: String,
  task: Option<task::JoinHandle<()>>,
}

impl Agent {
  async fn approval(
    &self,
    event_sender: &UnboundedSender<AgentEvent>,
    tool_call: &ToolRequest,
    policy: ApprovalPolicy,
  ) -> Result<ToolApproval> {
    if self.config.yolo || policy == ApprovalPolicy::None {
      return Ok(ToolApproval::Approved);
    }

    let (request, response_receiver) = ApprovalRequest::new(tool_call.clone());

    event_sender.send(AgentEvent::ToolApprovalRequest(request))?;

    Ok(response_receiver.await.unwrap_or(ToolApproval::Denied))
  }

  pub fn interrupt(&mut self) {
    if let Some(task) = self.task.take() {
      task.abort();
    }
  }

  pub fn new(
    config: AgentConfig,
    provider: Arc<dyn Provider>,
    system: String,
  ) -> Self {
    Self {
      config,
      provider,
      system,
      task: None,
    }
  }

  pub fn spawn(
    &mut self,
    conversation: Conversation,
  ) -> UnboundedReceiver<AgentEvent> {
    self.interrupt();

    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    let agent = Self {
      config: self.config.clone(),
      provider: self.provider.clone(),
      system: self.system.clone(),
      task: None,
    };

    self.task = Some(tokio::spawn(async move {
      if let Err(error) = agent.stream(&event_sender, conversation).await {
        let _ = event_sender.send(AgentEvent::Error(format!("{error:#}")));
      }
    }));

    event_receiver
  }

  async fn stream(
    &self,
    event_sender: &UnboundedSender<AgentEvent>,
    mut conversation: Conversation,
  ) -> Result {
    let tools = self.config.tools.definitions();

    let mut usage = Usage::default();

    for message in conversation.interrupt_pending_calls() {
      event_sender.send(AgentEvent::Message(message))?;
    }

    loop {
      let sink = ProviderSink {
        event_sender: event_sender.clone(),
      };

      let request = Request {
        messages: conversation.messages(),
        model: &self.config.model,
        system: (!self.system.is_empty()).then_some(self.system.as_str()),
        tools: &tools,
      };

      let response = self.provider.stream(request, &sink).await?;

      usage += response.usage;

      event_sender.send(AgentEvent::Usage(usage))?;

      let message = response.message;

      if !message.content.is_empty() {
        sink.update(MessageUpdate::Complete(message.clone()))?;
      }

      match response.finish_reason {
        Some(FinishReason::Length) => {
          bail!("completion truncated: output token limit reached");
        }
        Some(FinishReason::ContentFilter) => {
          bail!("completion filtered by the provider");
        }
        _ => {}
      }

      if message.content.is_empty() {
        break;
      }

      let tool_calls = message
        .content
        .iter()
        .filter_map(|content| match content {
          AssistantContent::ToolCall(call) => {
            Some(self.config.tools.prepare(call.clone()))
          }
          _ => None,
        })
        .collect::<Result<Vec<_>>>()?;

      let message = Message::Agent(message);

      for message in conversation
        .push(message.clone())
        .into_iter()
        .chain(once(message))
      {
        event_sender.send(AgentEvent::Message(message))?;
      }

      if tool_calls.is_empty() {
        break;
      }

      for (tool_call, tool) in tool_calls {
        let result = match self
          .approval(event_sender, &tool_call, tool.approval)
          .await?
        {
          ToolApproval::Approved => tool.future.await,
          ToolApproval::Denied => ToolResult {
            stderr: Some("permission denied".into()),
            ..Default::default()
          },
        };

        let message = Message::User(vec![UserMessageContent::ToolResult {
          call: tool_call.id,
          name: tool_call.function.name,
          result,
        }]);

        for message in conversation
          .push(message.clone())
          .into_iter()
          .chain(once(message))
        {
          event_sender.send(AgentEvent::Message(message))?;
        }
      }
    }

    event_sender.send(AgentEvent::Done)?;

    Ok(())
  }
}

impl Drop for Agent {
  fn drop(&mut self) {
    self.interrupt();
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[derive(Debug)]
  struct TestProvider {
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
    responses: Mutex<VecDeque<TestResponse>>,
  }

  impl Provider for TestProvider {
    fn stream<'a>(
      &'a self,
      request: Request<'a>,
      sink: &'a ProviderSink,
    ) -> BoxFuture<'a, Result<ProviderResponse>> {
      Box::pin(async move {
        assert_eq!(request.tools, [TestTool.definition().unwrap()]);

        self
          .requests
          .lock()
          .unwrap()
          .push(request.messages.to_vec());

        let response = self.responses.lock().unwrap().pop_front().unwrap();

        sink.update_many(response.updates)?;

        Ok(ProviderResponse {
          message: response.message,
          ..Default::default()
        })
      })
    }
  }

  #[derive(Debug, Default)]
  struct TestResponse {
    message: AgentMessage,
    updates: Vec<MessageUpdate>,
  }

  #[derive(Debug)]
  struct TestTool;

  impl Tool for TestTool {
    fn definition(&self) -> Result<ToolDefinition> {
      Ok(ToolDefinition {
        description: "bar".into(),
        name: ToolName::new("foo")?,
        parameters: json!({"type": "string"}),
      })
    }

    fn prepare(&self, arguments: Value) -> Result<ToolCall> {
      let content = serde_json::from_value::<String>(arguments)?;

      Ok(ToolCall {
        approval: ApprovalPolicy::Required,
        future: Box::pin(async move {
          ToolResult {
            content: Some(content),
            outcome: ToolOutcome::Success,
            ..Default::default()
          }
        }),
      })
    }
  }

  struct TestAgent {
    agent: Agent,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
  }

  impl TestAgent {
    fn new(responses: Vec<TestResponse>, yolo: bool) -> Self {
      let requests = Arc::new(Mutex::new(Vec::new()));

      let mut tools = ToolRegistry::default();

      tools.register(Arc::new(TestTool)).unwrap();

      let agent = Agent::new(
        AgentConfig {
          model: "mock:local".parse().unwrap(),
          tools,
          yolo,
        },
        Arc::new(TestProvider {
          requests: requests.clone(),
          responses: Mutex::new(responses.into()),
        }),
        String::new(),
      );

      Self { agent, requests }
    }
  }

  #[tokio::test]
  async fn approvals_can_be_denied() {
    let tool_call = AssistantContent::tool_call(
      "foo",
      ToolName::new("foo").unwrap(),
      json!("bar"),
    );

    let TestAgent {
      mut agent,
      requests,
    } = TestAgent::new(
      vec![
        TestResponse {
          message: vec![tool_call.clone()].into(),
          ..Default::default()
        },
        TestResponse::default(),
      ],
      false,
    );

    let mut events = agent.spawn(
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])].into(),
    );

    assert_eq!(
      events.recv().await.unwrap(),
      AgentEvent::Usage(Usage::default()),
    );

    assert_eq!(
      events.recv().await.unwrap(),
      AgentEvent::Update(MessageUpdate::Complete(AgentMessage::default())),
    );

    assert_eq!(
      events.recv().await.unwrap(),
      AgentEvent::Message(Message::Agent(vec![tool_call.clone()].into()))
    );

    let request = match events.recv().await.unwrap() {
      AgentEvent::ToolApprovalRequest(request) => request,
      event => panic!("expected approval request, got {event:?}"),
    };

    request.respond(ToolApproval::Denied);

    let tool_result = ToolResult {
      stderr: Some("permission denied".into()),
      ..Default::default()
    };

    assert_eq!(
      events.recv().await.unwrap(),
      AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::new("foo"),
          name: ToolName::new("foo").unwrap(),
          result: tool_result.clone(),
        }
      ])),
    );

    assert_eq!(
      events.recv().await.unwrap(),
      AgentEvent::Usage(Usage::default()),
    );

    assert_eq!(events.recv().await.unwrap(), AgentEvent::Done);

    let requests = requests.lock().unwrap();

    assert_eq!(
      *requests,
      [
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
        vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::Agent(vec![tool_call].into()),
          Message::User(vec![UserMessageContent::ToolResult {
            call: CallId::new("foo"),
            name: ToolName::new("foo").unwrap(),
            result: tool_result.clone(),
          }]),
        ],
      ],
    );
  }

  #[tokio::test]
  async fn loops_until_provider_returns_no_tool_calls() {
    let reasoning = Reasoning {
      content: vec![ReasoningContent::Encrypted("foo".into())],
      id: Some("bar".into()),
      issuer: "mock".into(),
    };

    let message = AgentMessage {
      content: vec![
        AssistantContent::text("foo"),
        AssistantContent::Reasoning(Reasoning {
          content: vec![ReasoningContent::Text {
            text: "bar".into(),
            signature: Some("baz".into()),
          }],
          id: Some("qux".into()),
          issuer: "mock".into(),
        }),
        AssistantContent::Reasoning(reasoning),
        AssistantContent::ToolCall(ToolRequest {
          additional_params: Some(json!({"foo": "bar"})),
          function: ToolFunction {
            arguments: json!("bar"),
            name: ToolName::new("foo").unwrap(),
          },
          id: CallId::Provider {
            call_id: "qux".into(),
            item_id: Some("baz".into()),
          },
          signature: Some("quux".into()),
        }),
        AssistantContent::text("baz"),
      ],
      id: Some("quuz".into()),
    };

    let update = |delta: &str| MessageUpdate::Text {
      delta: delta.into(),
      index: 0,
    };

    let complete = AgentMessage::from(vec![AssistantContent::text("bar")]);

    let mut test_agent = TestAgent::new(
      vec![
        TestResponse {
          message: message.clone(),
          updates: vec![update("quux")],
        },
        TestResponse {
          message: complete.clone(),
          updates: vec![update("bar")],
        },
      ],
      true,
    );

    let user = Message::User(vec![UserMessageContent::Text("foo".into())]);

    let mut events = test_agent.agent.spawn(vec![user].into());

    let mut received = Vec::new();

    while events.recv_many(&mut received, usize::MAX).await != 0 {}

    let result = ToolResult {
      content: Some("bar".into()),
      outcome: ToolOutcome::Success,
      ..Default::default()
    };

    let tool_result = Message::User(vec![UserMessageContent::ToolResult {
      call: CallId::Provider {
        call_id: "qux".into(),
        item_id: Some("baz".into()),
      },
      name: ToolName::new("foo").unwrap(),
      result: result.clone(),
    }]);

    let requests = test_agent.requests.lock().unwrap();

    assert_eq!(requests.len(), 2);

    let messages = serde_json::from_str::<Vec<Message>>(
      &serde_json::to_string(&requests[1]).unwrap(),
    )
    .unwrap();

    assert_eq!(
      messages,
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::Agent(message.clone()),
        tool_result.clone(),
      ]
    );

    let mut preview = message.clone();

    preview.content.remove(3);

    assert_eq!(
      received,
      [
        AgentEvent::Update(update("quux")),
        AgentEvent::Usage(Usage::default()),
        AgentEvent::Update(MessageUpdate::Complete(preview)),
        AgentEvent::Message(Message::Agent(message)),
        AgentEvent::Message(tool_result),
        AgentEvent::Update(update("bar")),
        AgentEvent::Usage(Usage::default()),
        AgentEvent::Update(MessageUpdate::Complete(complete.clone())),
        AgentEvent::Message(Message::Agent(complete)),
        AgentEvent::Done,
      ],
    );
  }
}
