use super::*;

#[derive(Clone, Debug)]
pub struct Agent {
  config: AgentConfig,
  context_strategy: Arc<dyn ContextStrategy>,
  provider: Arc<dyn Provider>,
  system: String,
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

  async fn execute(
    &self,
    run: &RunContext,
    response: ResponseId,
    tool_call: &ToolRequest,
    tool: ToolCall,
  ) -> Result<ToolResult> {
    Ok(
      match self
        .approval(&run.event_sender, tool_call, tool.approval)
        .await?
      {
        ToolApproval::Approved => {
          let context = ToolContext::new(run, response, tool_call);

          let guard = ToolGuard::new(context.clone())?;

          let result = tool.execute(context).await;

          guard.finish(&result);

          result
        }
        ToolApproval::Denied => ToolResult {
          outcome: ToolOutcome::Denied,
          stderr: Some("permission denied".into()),
          ..Default::default()
        },
      },
    )
  }

  pub fn new(
    config: AgentConfig,
    provider: Arc<dyn Provider>,
    system: String,
  ) -> Self {
    Self {
      config,
      context_strategy: Arc::new(FullContext),
      provider,
      system,
    }
  }

  async fn prepare_context<'a>(
    &'a self,
    conversation: &'a Conversation,
    tools: &'a [ToolDefinition],
  ) -> Result<RequestContext<'a>> {
    let context = self
      .context_strategy
      .prepare(Request {
        messages: conversation.messages(),
        model: &self.config.model,
        system: (!self.system.is_empty()).then_some(self.system.as_str()),
        tools,
      })
      .await?;

    context.request(&self.config.model).validate()?;

    Ok(context)
  }

  pub fn spawn(&self, conversation: Conversation) -> AgentRun {
    self.spawn_inner(conversation, None)
  }

  fn spawn_inner(
    &self,
    conversation: Conversation,
    deadline: Option<Instant>,
  ) -> AgentRun {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    let run = RunContext::new(event_sender, deadline);

    let cancellation = run.cancellation.clone();

    let id = run.id;

    let state = RunState::new(conversation.clone());

    let agent = self.clone();

    let task = tokio::spawn(async move {
      let _guard = run.cancellation.clone().drop_guard();

      tokio::select! {
        biased;
        outcome = run.interrupted() => Ok(outcome),
        result = agent.stream(&run, conversation) => result,
      }
    });

    AgentRun {
      cancellation,
      event_receiver,
      id,
      state,
      task,
    }
  }

  pub fn spawn_with_deadline(
    &self,
    conversation: Conversation,
    deadline: Instant,
  ) -> AgentRun {
    self.spawn_inner(conversation, Some(deadline))
  }

  async fn stream(
    &self,
    run: &RunContext,
    mut conversation: Conversation,
  ) -> Result<RunOutcome> {
    let event_sender = &run.event_sender;

    let mut rounds = 0;

    for message in conversation.interrupt_pending_calls()? {
      event_sender.send(AgentEvent::Message(message))?;
    }

    loop {
      if rounds >= self.config.max_rounds {
        return Ok(RunOutcome::Limited(RunLimit::Rounds {
          max: self.config.max_rounds,
        }));
      }

      let tools = self.config.tools.definitions();

      let context = self.prepare_context(&conversation, &tools).await?;

      let request = context.request(&self.config.model);

      let tools = request
        .tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect::<BTreeSet<_>>();

      let sink = ProviderSink::new(event_sender.clone());

      rounds += 1;

      let id = run.response(rounds);

      let guard = ResponseGuard::new(event_sender, id)?;

      let response = self.provider.stream(request, &sink).await;

      guard.finish(&response);

      drop(context);

      let response = response?;

      let message = response.message;

      let outcome = match response.finish_reason {
        Some(FinishReason::Length) => {
          Some(RunOutcome::Limited(RunLimit::OutputTokens))
        }
        Some(FinishReason::ContentFilter) => Some(RunOutcome::Failed(
          "completion filtered by the provider".into(),
        )),
        _ => None,
      };

      if let Some(outcome) = outcome {
        let mut message = message;

        message
          .content
          .retain(|content| !matches!(content, AssistantContent::ToolCall(_)));

        event_sender.send(AgentEvent::Message(Message::Agent(message)))?;

        return Ok(outcome);
      }

      if message.content.is_empty() {
        break;
      }

      let tool_calls = message
        .content
        .iter()
        .filter_map(|content| match content {
          AssistantContent::ToolCall(call) => Some(call.clone()),
          _ => None,
        })
        .collect::<Vec<_>>();

      let message = Message::Agent(message);

      conversation.push(message.clone());

      event_sender.send(AgentEvent::Message(message))?;

      if tool_calls.is_empty() {
        break;
      }

      let tool_calls = tool_calls
        .into_iter()
        .map(|call| {
          let tool = tools
            .get(&call.function.name)
            .with_context(|| format!("unknown tool `{}`", call.function.name))
            .and_then(|_| self.config.tools.prepare(&call));

          (call, tool)
        })
        .collect::<Vec<_>>();

      let invalid = tool_calls.iter().any(|(_, tool)| tool.is_err());

      for (tool_call, tool) in tool_calls {
        let result = match tool {
          Ok(tool) if !invalid => {
            self.execute(run, id, &tool_call, tool).await?
          }
          Ok(_) => ToolResult {
            outcome: ToolOutcome::Skipped,
            stderr: Some("batch skipped: another tool call is invalid".into()),
            ..Default::default()
          },
          Err(error) => ToolResult {
            stderr: Some(format!("{error:#}")),
            ..Default::default()
          },
        };

        let message = Message::User(vec![UserMessageContent::ToolResult {
          call: tool_call.id,
          name: tool_call.function.name,
          result,
        }]);

        conversation.push(message.clone());

        event_sender.send(AgentEvent::Message(message))?;
      }
    }

    Ok(RunOutcome::Completed)
  }

  #[must_use]
  pub fn with_context_strategy(
    self,
    context_strategy: Arc<dyn ContextStrategy>,
  ) -> Self {
    Self {
      context_strategy,
      ..self
    }
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

        if response.pending {
          return std::future::pending().await;
        }

        Ok(ProviderResponse {
          message: response.message,
          usage: response.usage,
          ..Default::default()
        })
      })
    }
  }

  #[derive(Debug, Default)]
  struct TestResponse {
    message: AgentMessage,
    pending: bool,
    updates: Vec<MessageUpdate>,
    usage: Usage,
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

      Ok(ToolCall::new(ApprovalPolicy::Required, |_| async move {
        ToolResult {
          content: Some(content),
          outcome: ToolOutcome::Success,
          ..Default::default()
        }
      }))
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
          max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
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

    let TestAgent { agent, requests } = TestAgent::new(
      vec![
        TestResponse {
          message: vec![tool_call.clone()].into(),
          ..Default::default()
        },
        TestResponse::default(),
      ],
      false,
    );

    let result = agent
      .spawn(
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
          .into(),
      )
      .join()
      .await;

    let tool_result = ToolResult {
      outcome: ToolOutcome::Denied,
      stderr: Some("permission denied".into()),
      ..Default::default()
    };

    assert_eq!(result.outcome, RunOutcome::Completed);

    let requests = requests.lock().unwrap();

    assert_eq!(result.conversation.messages(), requests[1]);

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
  async fn interruption_preserves_streamed_protocol() {
    let reasoning = Reasoning {
      content: vec![ReasoningContent::Text {
        text: "foo".into(),
        signature: Some("bar".into()),
      }],
      id: Some("baz".into()),
      issuer: "foo".into(),
    };

    let encrypted = Reasoning {
      content: vec![ReasoningContent::Encrypted("qux".into())],
      id: Some("quux".into()),
      issuer: "foo".into(),
    };

    let update = MessageUpdate::ReasoningDelta {
      delta: "foo".into(),
      index: 0,
    };

    let test_agent = TestAgent::new(
      vec![TestResponse {
        pending: true,
        updates: vec![
          update.clone(),
          MessageUpdate::Content {
            index: 0,
            content: AssistantContent::Reasoning(reasoning.clone()),
          },
          MessageUpdate::Content {
            index: 1,
            content: AssistantContent::Reasoning(encrypted.clone()),
          },
          MessageUpdate::Text {
            delta: "quuz".into(),
            index: 2,
          },
        ],
        ..Default::default()
      }],
      true,
    );

    let mut run = test_agent.agent.spawn(Conversation::default());

    let id = ResponseId {
      round: 1,
      run: run.id(),
    };

    assert_eq!(run.recv().await, Some(AgentEvent::ResponseStarted(id)));
    assert_eq!(run.recv().await, Some(AgentEvent::Update(update)));

    let message = Message::Agent(
      vec![
        AssistantContent::Reasoning(reasoning),
        AssistantContent::Reasoning(encrypted),
        AssistantContent::text("quuz"),
      ]
      .into(),
    );

    assert_eq!(
      run.cancel_and_join().await,
      RunResult {
        conversation: vec![message.clone()].into(),
        entries: vec![RunEntry::Message(message)],
        outcome: RunOutcome::Cancelled,
        responses: vec![ResponseResult {
          finish_reason: None,
          id,
          outcome: ResponseOutcome::Interrupted,
          usage: Usage::default(),
        }],
        usage: Usage::default(),
      },
    );
  }

  #[tokio::test]
  #[allow(clippy::too_many_lines)]
  async fn loops_until_provider_returns_no_tool_calls() {
    let usage = |tokens| Usage {
      total_tokens: Some(tokens),
      ..Default::default()
    };

    let call = CallId::Provider {
      call_id: "qux".into(),
      item_id: Some("baz".into()),
    };

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
          id: call.clone(),
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

    let test_agent = TestAgent::new(
      vec![
        TestResponse {
          message: message.clone(),
          updates: vec![update("quux")],
          usage: usage(2),
          ..Default::default()
        },
        TestResponse {
          message: complete.clone(),
          updates: vec![update("bar")],
          usage: usage(3),
          ..Default::default()
        },
      ],
      true,
    );

    let user = Message::User(vec![UserMessageContent::Text("foo".into())]);

    let mut events = test_agent.agent.spawn(vec![user].into());

    let response = |round| ResponseId {
      round,
      run: events.id(),
    };

    let first = response(1);
    let second = response(2);

    let mut received = Vec::new();

    while let Some(event) = events.recv().await {
      received.push(event);
    }

    let result = events.join().await;

    assert_eq!(result.outcome, RunOutcome::Completed);

    let tool_result = ToolResult {
      content: Some("bar".into()),
      outcome: ToolOutcome::Success,
      ..Default::default()
    };

    let tool_message = Message::User(vec![UserMessageContent::ToolResult {
      call: call.clone(),
      name: ToolName::new("foo").unwrap(),
      result: tool_result.clone(),
    }]);

    let requests = test_agent.requests.lock().unwrap();

    assert_eq!(requests.len(), 2);

    assert_eq!(result.usage, usage(5));
    assert_eq!(
      result.conversation.messages(),
      requests[1]
        .iter()
        .cloned()
        .chain([Message::Agent(complete.clone())])
        .collect::<Vec<_>>(),
    );

    let messages = serde_json::from_str::<Vec<Message>>(
      &serde_json::to_string(&requests[1]).unwrap(),
    )
    .unwrap();

    assert_eq!(
      messages,
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::Agent(message.clone()),
        tool_message.clone(),
      ]
    );

    assert_eq!(
      received,
      [
        AgentEvent::ResponseStarted(first),
        AgentEvent::Update(update("quux")),
        AgentEvent::ResponseFinished(ResponseResult {
          finish_reason: None,
          id: first,
          outcome: ResponseOutcome::Completed,
          usage: usage(2),
        }),
        AgentEvent::Message(Message::Agent(message)),
        AgentEvent::ToolStarted {
          call: call.clone(),
          name: ToolName::new("foo").unwrap(),
          response: first
        },
        AgentEvent::ToolFinished {
          call,
          response: first,
          result: tool_result
        },
        AgentEvent::Message(tool_message),
        AgentEvent::ResponseStarted(second),
        AgentEvent::Update(update("bar")),
        AgentEvent::ResponseFinished(ResponseResult {
          finish_reason: None,
          id: second,
          outcome: ResponseOutcome::Completed,
          usage: usage(3),
        }),
        AgentEvent::Message(Message::Agent(complete)),
      ],
    );
  }
}
