use super::*;

#[derive(Debug)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  loader: Loader,
  provider: Arc<dyn Provider>,
  settings: Settings,
  task: Option<task::JoinHandle<()>>,
  tool_registry: ToolRegistry,
}

impl Agent {
  async fn approval(
    &self,
    run_id: u64,
    tool_call: &ToolInvocation,
  ) -> Result<ToolApproval> {
    if self.settings.yolo || !tool_call.kind.requires_approval() {
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
      loader: Loader::new()?,
      provider,
      settings: settings.clone(),
      task: None,
      tool_registry: ToolRegistry::default(),
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
      tool_registry: self.tool_registry.clone(),
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

    loop {
      let mut sink = ProviderSink::new(
        self.event_sender.clone(),
        run_id,
        self.tool_registry.clone(),
      );

      let request = Request::with_system(
        self.settings.model.clone(),
        messages.clone(),
        system.clone(),
        self.tool_registry.clone(),
      );

      self.provider.stream(request, &mut sink).await?;

      let content = sink.finish();

      if content.is_empty() {
        break;
      }

      let tool_calls = content
        .iter()
        .filter_map(|content| match content {
          AgentMessageContent::Reasoning(_) | AgentMessageContent::Text(_) => {
            None
          }
          AgentMessageContent::ToolCall(invocation) => Some(invocation.clone()),
        })
        .collect::<Vec<_>>();

      messages.push(Message::Agent(content));

      if tool_calls.is_empty() {
        break;
      }

      for tool_call in tool_calls {
        let result = tool_call
          .kind
          .execute(self.approval(run_id, &tool_call).await?)
          .await;

        messages.push(result.message(tool_call.id.clone()));

        self.event_sender.send(Event::Agent {
          event: AgentEvent::ToolResult {
            id: tool_call.id,
            result,
          },
          run_id,
        })?;
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

      When using tools, omit `cwd` to use the current working directory. Do not invent absolute paths.
      ",
      self.loader.cwd.display(),
    }
    .trim_end()
    .to_string();

    Ok(if agents.is_empty() {
      formatdoc! {
        "
        {}

        {context}
        ",
        SYSTEM_PROMPT.as_str(),
      }
      .trim_end()
      .to_string()
    } else {
      formatdoc! {
        "
        {}

        {context}

        {agents}
        ",
        SYSTEM_PROMPT.as_str(),
      }
      .trim_end()
      .to_string()
    })
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*, serde_json::json, std::collections::VecDeque, tempfile::TempDir,
  };

  #[derive(Debug)]
  enum Output {
    Delta(&'static str),
    Reasoning(&'static str),
    ReasoningDelta(&'static str),
    ToolCall,
  }

  #[derive(Debug)]
  struct TestProvider {
    outputs: Mutex<VecDeque<Vec<Output>>>,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
  }

  #[async_trait]
  impl Provider for TestProvider {
    async fn stream(
      &self,
      request: Request,
      sink: &mut ProviderSink,
    ) -> Result {
      self.requests.lock().unwrap().push(request.messages);

      for output in self.outputs.lock().unwrap().pop_front().unwrap() {
        match output {
          Output::Delta(delta) => sink.delta(delta)?,
          Output::ReasoningDelta(delta) => {
            sink.reasoning_delta(None, delta)?;
          }
          Output::Reasoning(reasoning) => {
            sink.reasoning(Reasoning::new(reasoning))?;
          }
          Output::ToolCall => sink.tool_call(RawToolCall {
            arguments: json!({
              "arguments": ["bar"],
              "cwd": null,
              "program": "echo",
            }),
            id: "foo".into(),
            name: "command".into(),
          })?,
        }
      }

      Ok(())
    }
  }

  struct TestAgent {
    agent: Agent,
    directory: TempDir,
    events: UnboundedReceiver<Event>,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
  }

  impl TestAgent {
    fn new(outputs: Vec<Vec<Output>>, yolo: bool) -> Self {
      let (event_sender, events) = mpsc::unbounded_channel();

      let directory = tempfile::tempdir().unwrap();

      let requests = Arc::new(Mutex::new(Vec::new()));

      let agent = Agent {
        event_sender,
        loader: Loader::with_cwd(directory.path()),
        provider: Arc::new(TestProvider {
          outputs: Mutex::new(outputs.into()),
          requests: requests.clone(),
        }),
        settings: Settings {
          model: "mock:local".parse().unwrap(),
          prompt: None,
          yolo,
        },
        task: None,
        tool_registry: ToolRegistry::default(),
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
  async fn command_tools_wait_for_approval() {
    let TestAgent {
      agent,
      directory: _directory,
      mut events,
      requests,
    } = TestAgent::new(
      vec![vec![Output::ToolCall], vec![Output::Delta("done")]],
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
        event: AgentEvent::ToolCall(ToolInvocation {
          id: "foo".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            arguments: vec!["bar".into()],
            cwd: None,
            program: "echo".into(),
          }),
        }),
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

    request.deny();

    task.await.unwrap();

    let tool_result = ToolResult::error("permission denied");

    assert_eq!(
      events.recv().await.unwrap(),
      Event::Agent {
        event: AgentEvent::ToolResult {
          id: "foo".into(),
          result: tool_result.clone(),
        },
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
          Message::Agent(vec![AgentMessageContent::ToolCall(ToolInvocation {
            id: "foo".into(),
            kind: ToolInvocationKind::Command(CommandTool {
              arguments: vec!["bar".into()],
              cwd: None,
              program: "echo".into(),
            }),
          })]),
          tool_result.message("foo"),
        ],
      ],
    );
  }

  #[tokio::test]
  async fn loops_until_provider_returns_no_tool_calls() {
    let mut test_agent = TestAgent::new(
      vec![vec![Output::ToolCall], vec![Output::Delta("done")]],
      true,
    );

    test_agent
      .agent
      .stream(
        0,
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
      )
      .await
      .unwrap();

    let requests = test_agent.requests.lock().unwrap();

    let tool_result = ToolResult::command(Some(0), "bar\n", "");

    assert_eq!(
      *requests,
      [
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
        vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::Agent(vec![AgentMessageContent::ToolCall(ToolInvocation {
            id: "foo".into(),
            kind: ToolInvocationKind::Command(CommandTool {
              arguments: vec!["bar".into()],
              cwd: None,
              program: "echo".into(),
            }),
          })]),
          tool_result.message("foo"),
        ],
      ],
    );

    let mut events = Vec::new();

    while let Ok(event) = test_agent.events.try_recv() {
      events.push(event);
    }

    assert_eq!(
      events,
      [
        Event::Agent {
          event: AgentEvent::ToolCall(ToolInvocation {
            id: "foo".into(),
            kind: ToolInvocationKind::Command(CommandTool {
              arguments: vec!["bar".into()],
              cwd: None,
              program: "echo".into(),
            }),
          }),
          run_id: 0,
        },
        Event::Agent {
          event: AgentEvent::ToolResult {
            id: "foo".into(),
            result: tool_result,
          },
          run_id: 0,
        },
        Event::Agent {
          event: AgentEvent::Delta("done".into()),
          run_id: 0,
        },
        Event::Agent {
          event: AgentEvent::Done,
          run_id: 0,
        },
      ],
    );
  }

  #[tokio::test]
  async fn preserves_ordered_agent_content() {
    let test_agent = TestAgent::new(
      vec![
        vec![Output::Delta("foo"), Output::ToolCall, Output::Delta("baz")],
        vec![Output::Delta("done")],
      ],
      true,
    );

    test_agent
      .agent
      .stream(
        0,
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
      )
      .await
      .unwrap();

    let requests = test_agent.requests.lock().unwrap();

    let tool_result = ToolResult::command(Some(0), "bar\n", "");

    assert_eq!(
      *requests,
      [
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
        vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::Agent(vec![
            AgentMessageContent::Text("foo".into()),
            AgentMessageContent::ToolCall(ToolInvocation {
              id: "foo".into(),
              kind: ToolInvocationKind::Command(CommandTool {
                arguments: vec!["bar".into()],
                cwd: None,
                program: "echo".into(),
              }),
            }),
            AgentMessageContent::Text("baz".into()),
          ]),
          tool_result.message("foo"),
        ],
      ],
    );
  }

  #[tokio::test]
  async fn preserves_reasoning_with_tool_calls() {
    let test_agent = TestAgent::new(
      vec![
        vec![Output::ReasoningDelta("baz"), Output::ToolCall],
        vec![Output::Delta("done")],
      ],
      true,
    );

    test_agent
      .agent
      .stream(
        0,
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
      )
      .await
      .unwrap();

    let requests = test_agent.requests.lock().unwrap();

    let tool_result = ToolResult::command(Some(0), "bar\n", "");

    assert_eq!(
      *requests,
      [
        vec![Message::User(vec![UserMessageContent::Text("foo".into())])],
        vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::Agent(vec![
            AgentMessageContent::Reasoning("baz".into()),
            AgentMessageContent::ToolCall(ToolInvocation {
              id: "foo".into(),
              kind: ToolInvocationKind::Command(CommandTool {
                arguments: vec!["bar".into()],
                cwd: None,
                program: "echo".into(),
              }),
            }),
          ]),
          tool_result.message("foo"),
        ],
      ],
    );
  }

  #[tokio::test]
  async fn streams_reasoning() {
    let mut test_agent = TestAgent::new(
      vec![vec![
        Output::ReasoningDelta("foo"),
        Output::Reasoning("foo"),
        Output::Delta("bar"),
      ]],
      true,
    );

    test_agent
      .agent
      .stream(
        0,
        vec![Message::User(vec![UserMessageContent::Text("baz".into())])],
      )
      .await
      .unwrap();

    let mut events = Vec::new();

    while let Ok(event) = test_agent.events.try_recv() {
      events.push(event);
    }

    assert_eq!(
      events,
      [
        Event::Agent {
          event: AgentEvent::ReasoningDelta("foo".into()),
          run_id: 0,
        },
        Event::Agent {
          event: AgentEvent::Delta("bar".into()),
          run_id: 0,
        },
        Event::Agent {
          event: AgentEvent::Done,
          run_id: 0,
        },
      ],
    );
  }

  #[test]
  fn system_prompt() {
    fn context(directory: &Path) -> String {
      formatdoc! {
        "
        Current working directory: {}

        When using tools, omit `cwd` to use the current working directory. Do not invent absolute paths.
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
