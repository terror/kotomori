use super::*;

#[derive(Debug)]
pub(crate) struct Agent {
  pub(crate) config: AgentConfig,
  pub(crate) provider: Arc<dyn Provider>,
  pub(crate) system: String,
  pub(crate) task: Option<task::JoinHandle<()>>,
}

impl Agent {
  async fn approval(
    &self,
    event_sender: &UnboundedSender<AgentEvent>,
    tool_call: &ToolInvocation,
  ) -> Result<ToolApproval> {
    if self.config.yolo || tool_call.kind.approval() == ApprovalPolicy::None {
      return Ok(ToolApproval::Approved);
    }

    let (request, response_receiver) = ApprovalRequest::new(tool_call.clone());

    event_sender.send(AgentEvent::ToolApprovalRequest(request))?;

    Ok(response_receiver.await.unwrap_or(ToolApproval::Denied))
  }

  pub(crate) fn interrupt(&mut self) {
    if let Some(task) = self.task.take() {
      task.abort();
    }
  }

  pub(crate) fn spawn(
    &mut self,
    messages: Vec<Message>,
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
      if let Err(error) = agent.stream(&event_sender, messages).await {
        let _ = event_sender.send(AgentEvent::Error(error.to_string()));
      }
    }));

    event_receiver
  }

  async fn stream(
    &self,
    event_sender: &UnboundedSender<AgentEvent>,
    mut messages: Vec<Message>,
  ) -> Result {
    let tools = ToolInvocationKind::definitions();

    loop {
      let sink = ProviderSink {
        event_sender: event_sender.clone(),
      };

      let request = Request {
        messages: &messages,
        model: &self.config.model,
        system: (!self.system.is_empty()).then_some(self.system.as_str()),
        tools: &tools,
      };

      let message = self.provider.stream(request, &sink).await?;

      if message.content.is_empty() {
        break;
      }

      let tool_calls = message
        .content
        .iter()
        .filter_map(|content| match content {
          AssistantContent::ToolCall(call) => {
            Some(ToolInvocationKind::decode(call.clone()))
          }
          _ => None,
        })
        .collect::<Result<Vec<_>>>()?;

      let message = Message::Agent(message);

      event_sender.send(AgentEvent::Message(message.clone()))?;

      messages.push(message);

      if tool_calls.is_empty() {
        break;
      }

      for tool_call in tool_calls {
        let result = match self.approval(event_sender, &tool_call).await? {
          ToolApproval::Approved => {
            self.config.tool_executor.execute(&tool_call.kind).await
          }
          ToolApproval::Denied => ToolResult {
            stderr: Some("permission denied".into()),
            ..Default::default()
          },
        };

        let message = Message::User(vec![UserMessageContent::ToolResult {
          call: tool_call.protocol.id,
          name: tool_call.protocol.function.name,
          result,
        }]);

        event_sender.send(AgentEvent::Message(message.clone()))?;

        messages.push(message);
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
  use {
    super::*, serde_json::json, std::collections::VecDeque, tempfile::TempDir,
  };

  const COMMAND_OUTPUT: &str = if cfg!(windows) { "bar\r\n" } else { "bar\n" };

  #[derive(Debug)]
  struct TestProvider {
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
    responses: Mutex<VecDeque<TestResponse>>,
  }

  #[derive(Debug, Default)]
  struct TestResponse {
    message: AgentMessage,
    updates: Vec<MessageUpdate>,
  }

  impl Provider for TestProvider {
    fn stream<'a>(
      &'a self,
      request: Request<'a>,
      sink: &'a ProviderSink,
    ) -> BoxFuture<'a, Result<AgentMessage>> {
      Box::pin(async move {
        assert_eq!(request.tools, ToolInvocationKind::definitions());

        self
          .requests
          .lock()
          .unwrap()
          .push(request.messages.to_vec());

        let response = self.responses.lock().unwrap().pop_front().unwrap();

        sink.update_many(response.updates)?;

        Ok(response.message)
      })
    }
  }

  struct TestAgent {
    agent: Agent,
    directory: TempDir,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
  }

  impl TestAgent {
    fn new(responses: Vec<TestResponse>, yolo: bool) -> Self {
      let directory = tempfile::tempdir().unwrap();

      let requests = Arc::new(Mutex::new(Vec::new()));

      let agent = Agent {
        config: AgentConfig {
          model: "mock:local".parse().unwrap(),
          tool_executor: ToolExecutor {
            command_executor: CommandExecutor::default(),
            directory: directory.path().into(),
          },
          yolo,
        },
        provider: Arc::new(TestProvider {
          requests: requests.clone(),
          responses: Mutex::new(responses.into()),
        }),
        system: String::new(),
        task: None,
      };

      Self {
        agent,
        directory,
        requests,
      }
    }
  }

  #[tokio::test]
  async fn command_tools_can_be_denied() {
    let tool_call = AssistantContent::tool_call(
      "foo",
      ToolName::new("command").unwrap(),
      json!({"command": "echo bar"}),
    );

    let TestAgent {
      mut agent,
      directory: _directory,
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

    let mut events =
      agent.spawn(vec![Message::User(vec![UserMessageContent::Text(
        "foo".into(),
      )])]);

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
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result: tool_result.clone(),
        }
      ])),
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
            call: CallId::from_wire("foo"),
            name: ToolName::new("command").unwrap(),
            result: tool_result.clone(),
          }]),
        ],
      ],
    );
  }

  #[tokio::test]
  async fn loops_until_provider_returns_no_tool_calls() {
    let reasoning = Reasoning::encrypted("foo")
      .with_id("bar".into())
      .sealed("mock");

    let message = AgentMessage {
      content: vec![
        AssistantContent::text("foo"),
        AssistantContent::Reasoning(
          Reasoning::new_with_signature("bar", Some("baz".into()))
            .with_id("qux".into())
            .sealed("mock"),
        ),
        AssistantContent::Reasoning(reasoning),
        AssistantContent::ToolCall(
          ::rig::message::ToolCall::from_dual_wire(
            "baz",
            "qux",
            ToolFunction {
              arguments: json!({"command": "echo bar", "cwd": null}),
              name: ToolName::new("command").unwrap(),
            },
          )
          .with_signature(Some("quux".into()))
          .with_additional_params(Some(json!({"foo": "bar"}))),
        ),
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

    let mut events = test_agent.agent.spawn(vec![user]);

    let mut received = Vec::new();

    while events.recv_many(&mut received, usize::MAX).await != 0 {}

    let result = ToolResult {
      exit_status: Some(0),
      outcome: ToolOutcome::Success,
      stdout: Some(COMMAND_OUTPUT.into()),
      ..Default::default()
    };

    let tool_result = Message::User(vec![UserMessageContent::ToolResult {
      call: CallId::from_dual_wire("baz", "qux"),
      name: ToolName::new("command").unwrap(),
      result: result.clone(),
    }]);

    let requests = test_agent.requests.lock().unwrap();

    assert_eq!(requests.len(), 2);

    let messages = serde_json::from_str::<Vec<Message>>(
      &serde_json::to_string(&requests[1]).unwrap(),
    )
    .unwrap();

    let request = CompletionRequest::from(&Request {
      messages: &messages,
      model: &"mock:foo".parse().unwrap(),
      system: None,
      tools: &[],
    });

    assert_eq!(
      request.chat_history,
      vec![
        RigMessage::user("foo"),
        RigMessage::Assistant {
          content: message.content.clone(),
          id: message.id.clone(),
        },
        RigMessage::tool_result(
          CallId::from_dual_wire("baz", "qux"),
          ToolName::new("command").unwrap(),
          result.message_content()
        ),
      ]
    );

    assert_eq!(
      received,
      [
        AgentEvent::Update(update("quux")),
        AgentEvent::Message(Message::Agent(message)),
        AgentEvent::Message(tool_result),
        AgentEvent::Update(update("bar")),
        AgentEvent::Message(Message::Agent(complete)),
        AgentEvent::Done,
      ],
    );
  }
}
