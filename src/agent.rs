use super::*;

#[derive(Debug)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  loader: Loader,
  provider: Arc<dyn Provider>,
  settings: Settings,
  task: Option<task::JoinHandle<()>>,
  tool_executor: ToolExecutor,
}

impl Agent {
  async fn approval(
    &self,
    run_id: u64,
    tool_call: &ToolInvocation,
  ) -> Result<ToolApproval> {
    if self.settings.yolo || tool_call.kind.approval() == ApprovalPolicy::None {
      return Ok(ToolApproval::Approved);
    }

    let (request, response_receiver) = ApprovalRequest::new(tool_call.clone());

    self.event_sender.send(Event::Agent {
      event: AgentEvent::ToolApprovalRequest(request),
      run_id,
    })?;

    Ok(response_receiver.await.unwrap_or(ToolApproval::Denied))
  }

  pub(crate) fn interrupt(&mut self) {
    if let Some(task) = self.task.take() {
      task.abort();
    }
  }

  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    settings: &Settings,
  ) -> Result<Self> {
    let provider = Arc::<dyn Provider>::try_from(settings.model.clone())?;

    Ok(Self {
      event_sender,
      loader: Loader::new(&settings.directory),
      provider,
      settings: settings.clone(),
      task: None,
      tool_executor: ToolExecutor {
        command_executor: CommandExecutor::default(),
        directory: settings.directory.clone(),
      },
    })
  }

  pub(crate) fn spawn(&mut self, run_id: u64, messages: Vec<Message>) {
    self.interrupt();

    let agent = Self {
      event_sender: self.event_sender.clone(),
      loader: self.loader.clone(),
      provider: self.provider.clone(),
      settings: self.settings.clone(),
      task: None,
      tool_executor: self.tool_executor.clone(),
    };

    self.task = Some(tokio::spawn(async move {
      if let Err(error) = agent.stream(run_id, messages).await {
        let _ = agent.event_sender.send(Event::Agent {
          event: AgentEvent::Error(error.to_string()),
          run_id,
        });
      }
    }));
  }

  async fn stream(&self, run_id: u64, mut messages: Vec<Message>) -> Result {
    let system = self.system_prompt()?;

    let tools = ToolInvocationKind::definitions();

    loop {
      let sink = ProviderSink {
        event_sender: self.event_sender.clone(),
        run_id,
      };

      let request = Request {
        messages: &messages,
        model: &self.settings.model,
        system: (!system.is_empty()).then_some(system.as_str()),
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

      self.event_sender.send(Event::Agent {
        event: AgentEvent::Message(message.clone()),
        run_id,
      })?;

      messages.push(message);

      if tool_calls.is_empty() {
        break;
      }

      for tool_call in tool_calls {
        let result = match self.approval(run_id, &tool_call).await? {
          ToolApproval::Approved => {
            self.tool_executor.execute(&tool_call.kind).await
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

        self.event_sender.send(Event::Agent {
          event: AgentEvent::Message(message.clone()),
          run_id,
        })?;

        messages.push(message);
      }
    }

    self.event_sender.send(Event::Agent {
      event: AgentEvent::Done,
      run_id,
    })?;

    Ok(())
  }

  fn system_prompt(&self) -> Result<String> {
    let agents = self.loader.load()?;

    let context = formatdoc! {
      "
      Current working directory: {}

      The command tool runs commands using the platform's system shell. Omit
      `cwd` to use the current working directory. Do not invent absolute paths.
      ",
      self.loader.cwd.display(),
    };

    Ok(
      [
        SYSTEM_PROMPT.as_str(),
        context.trim_end(),
        agents.trim_end(),
      ]
      .into_iter()
      .filter(|section| !section.is_empty())
      .collect::<Vec<_>>()
      .join("\n\n"),
    )
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
    events: UnboundedReceiver<Event>,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
  }

  impl TestAgent {
    fn new(responses: Vec<TestResponse>, yolo: bool) -> Self {
      let (event_sender, events) = mpsc::unbounded_channel();

      let directory = tempfile::tempdir().unwrap();

      let requests = Arc::new(Mutex::new(Vec::new()));

      let agent = Agent {
        event_sender,
        loader: Loader::new(directory.path()),
        provider: Arc::new(TestProvider {
          requests: requests.clone(),
          responses: Mutex::new(responses.into()),
        }),
        settings: Settings {
          directory: directory.path().into(),
          model: "mock:local".parse().unwrap(),
          prompt: None,
          yolo,
        },
        task: None,
        tool_executor: ToolExecutor {
          command_executor: CommandExecutor::default(),
          directory: directory.path().into(),
        },
      };

      Self {
        agent,
        directory,
        events,
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
      agent,
      directory: _directory,
      mut events,
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

    let task = tokio::spawn(async move {
      agent
        .stream(
          0,
          vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
        )
        .await
        .unwrap();
    });

    assert_eq!(
      events.recv().await.unwrap(),
      Event::Agent {
        event: AgentEvent::Message(Message::agent(vec![tool_call.clone()])),
        run_id: 0,
      }
    );

    let request = match events.recv().await.unwrap() {
      Event::Agent {
        event: AgentEvent::ToolApprovalRequest(request),
        run_id: 0,
      } => request,
      event => panic!("expected approval request, got {event:?}"),
    };

    request.respond(ToolApproval::Denied);

    task.await.unwrap();

    let tool_result = ToolResult {
      stderr: Some("permission denied".into()),
      ..Default::default()
    };

    assert_eq!(
      events.recv().await.unwrap(),
      Event::Agent {
        event: AgentEvent::Message(Message::User(vec![
          UserMessageContent::ToolResult {
            call: CallId::from_wire("foo"),
            name: ToolName::new("command").unwrap(),
            result: tool_result.clone(),
          }
        ])),
        run_id: 0,
      },
    );

    assert_eq!(
      events.recv().await.unwrap(),
      Event::Agent {
        event: AgentEvent::Done,
        run_id: 0,
      },
    );

    let requests = requests.lock().unwrap();

    assert_eq!(
      *requests,
      [
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
        vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::agent(vec![tool_call]),
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

    test_agent.agent.stream(0, vec![user]).await.unwrap();

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

    let events = std::iter::from_fn(|| test_agent.events.try_recv().ok())
      .collect::<Vec<_>>();

    assert_eq!(
      events,
      [
        AgentEvent::Update(update("quux")),
        AgentEvent::Message(Message::Agent(message)),
        AgentEvent::Message(tool_result),
        AgentEvent::Update(update("bar")),
        AgentEvent::Message(Message::Agent(complete)),
        AgentEvent::Done,
      ]
      .map(|event| Event::Agent { event, run_id: 0 }),
    );
  }

  #[test]
  fn system_prompt() {
    fn context(directory: &Path) -> String {
      formatdoc! {
        "
        Current working directory: {}

        The command tool runs commands using the platform's system shell. Omit
        `cwd` to use the current working directory. Do not invent absolute paths.
        ",
        directory.display(),
      }
      .trim_end()
      .to_string()
    }

    #[track_caller]
    fn case<F>(agents: Option<&str>, expected: F)
    where
      F: FnOnce(&Path, &Path) -> String,
    {
      let test_agent = TestAgent::new(Vec::new(), true);

      let agents_path = test_agent.directory.path().join("AGENTS.md");

      if let Some(agents) = agents {
        fs::write(&agents_path, agents).unwrap();
      }

      assert_eq!(
        test_agent.agent.system_prompt().unwrap(),
        expected(test_agent.directory.path(), &agents_path)
      );
    }

    case(None, |directory, _| {
      format!("{}\n\n{}", SYSTEM_PROMPT.as_str(), context(directory))
    });

    case(Some("foo\n"), |directory, agents_path| {
      format!(
        "{}\n\n{}\n\n{}:\nfoo",
        SYSTEM_PROMPT.as_str(),
        context(directory),
        agents_path.display()
      )
    });
  }
}
