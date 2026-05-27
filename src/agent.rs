use super::*;

#[derive(Debug)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  loader: Loader,
  model: Model,
  provider: Arc<dyn Provider>,
  task: Option<task::JoinHandle<()>>,
  yolo: bool,
}

impl Agent {
  async fn approval(&self, tool_call: &ToolInvocation) -> Result<ToolApproval> {
    if self.yolo || !tool_call.kind.requires_approval() {
      return Ok(ToolApproval::Approved);
    }

    let (request, response_receiver) = ApprovalRequest::new(tool_call.clone());

    self
      .event_sender
      .send(Event::ToolApprovalRequest(request))?;

    Ok(response_receiver.await.unwrap_or(ToolApproval::Denied))
  }

  pub(crate) fn interrupt(&mut self) {
    if let Some(task) = self.task.take() {
      task.abort();
    }
  }

  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    options: &Options,
  ) -> Result<Self> {
    let provider = Arc::<dyn Provider>::try_from(options.model.clone())?;

    Ok(Self {
      event_sender,
      loader: Loader::new()?,
      model: options.model.clone(),
      provider,
      task: None,
      yolo: options.yolo,
    })
  }

  pub(crate) fn spawn(&mut self, messages: Vec<Message>) {
    self.interrupt();

    let agent = Self {
      event_sender: self.event_sender.clone(),
      loader: self.loader.clone(),
      model: self.model.clone(),
      provider: self.provider.clone(),
      task: None,
      yolo: self.yolo,
    };

    self.task = Some(tokio::spawn(async move {
      if let Err(error) = agent.stream(messages).await {
        let _ = agent.event_sender.send(Event::Error(error.to_string()));
      }
    }));
  }

  async fn stream(&self, mut messages: Vec<Message>) -> Result {
    let system = self.loader.load()?;

    loop {
      let mut sink = ProviderSink::new(self.event_sender.clone());

      let request = if system.is_empty() {
        Request::new(self.model.clone(), messages.clone())
      } else {
        Request::with_system(
          self.model.clone(),
          messages.clone(),
          system.clone(),
        )
      };

      self.provider.stream(request, &mut sink).await?;

      let output = sink.finish();

      if !output.content.is_empty() {
        messages.push(Message::new(Role::Agent, output.content));
      }

      if output.tool_calls.is_empty() {
        break;
      }

      for tool_call in output.tool_calls {
        messages.push(tool_call.message());

        let approval = self.approval(&tool_call).await?;

        let result = tool_call.kind.execute(approval).await;

        messages.push(result.message(tool_call.id.clone()));

        self.event_sender.send(Event::AgentToolResult {
          id: tool_call.id,
          result,
        })?;
      }
    }

    self.event_sender.send(Event::AgentDone)?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct LoopProvider {
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
  }

  #[async_trait]
  impl Provider for LoopProvider {
    async fn stream(
      &self,
      request: Request,
      sink: &mut ProviderSink,
    ) -> Result {
      let index = {
        let mut requests = self.requests.lock().unwrap();

        let index = requests.len();

        requests.push(request.messages().cloned().collect());

        index
      };

      if index == 0 {
        sink.tool_call(RawToolCall::new(
          "foo",
          "command",
          json!({
            "arguments": ["bar"],
            "cwd": null,
            "program": "echo",
          }),
        ))?;
      } else {
        sink.delta("done")?;
      }

      Ok(())
    }
  }

  #[tokio::test]
  async fn command_tools_wait_for_approval() {
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();

    let directory = tempfile::tempdir().unwrap();

    let requests = Arc::new(Mutex::new(Vec::new()));

    let agent = Agent {
      event_sender,
      loader: Loader::with_cwd(directory.path()),
      model: "fake:local".parse().unwrap(),
      provider: Arc::new(LoopProvider {
        requests: requests.clone(),
      }),
      task: None,
      yolo: false,
    };

    let task = tokio::spawn(async move {
      agent
        .stream(vec![Message::new(Role::User, "foo")])
        .await
        .unwrap();
    });

    assert_eq!(
      event_receiver.recv().await.unwrap(),
      Event::AgentToolCall(ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::Command(CommandTool {
          arguments: vec!["bar".into()],
          cwd: None,
          program: "echo".into(),
        }),
      })
    );

    let request = match event_receiver.recv().await.unwrap() {
      Event::ToolApprovalRequest(request) => request,
      event => panic!("expected approval request, got {event:?}"),
    };

    request.deny();

    task.await.unwrap();

    let tool_result = ToolResult::error("permission denied");

    assert_eq!(
      event_receiver.recv().await.unwrap(),
      Event::AgentToolResult {
        id: "foo".into(),
        result: tool_result.clone(),
      },
    );

    let requests = requests.lock().unwrap();

    assert_eq!(
      *requests,
      [
        vec![Message::new(Role::User, "foo")],
        vec![
          Message::new(Role::User, "foo"),
          Message::tool_use(
            "foo",
            "command",
            json!({
              "arguments": ["bar"],
              "cwd": null,
              "program": "echo",
            }),
          ),
          tool_result.message("foo"),
        ],
      ],
    );
  }

  #[tokio::test]
  async fn loops_until_provider_returns_no_tool_calls() {
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();

    let directory = tempfile::tempdir().unwrap();

    let requests = Arc::new(Mutex::new(Vec::new()));

    let agent = Agent {
      event_sender,
      loader: Loader::with_cwd(directory.path()),
      model: "fake:local".parse().unwrap(),
      provider: Arc::new(LoopProvider {
        requests: requests.clone(),
      }),
      task: None,
      yolo: true,
    };

    agent
      .stream(vec![Message::new(Role::User, "foo")])
      .await
      .unwrap();

    let requests = requests.lock().unwrap();

    let tool_result = ToolResult::command(Some(0), "bar\n", "");

    assert_eq!(
      *requests,
      [
        vec![Message::new(Role::User, "foo")],
        vec![
          Message::new(Role::User, "foo"),
          Message::tool_use(
            "foo",
            "command",
            json!({
              "arguments": ["bar"],
              "cwd": null,
              "program": "echo",
            }),
          ),
          tool_result.message("foo"),
        ],
      ],
    );

    let mut events = Vec::new();

    while let Ok(event) = event_receiver.try_recv() {
      events.push(event);
    }

    assert_eq!(
      events,
      [
        Event::AgentToolCall(ToolInvocation {
          id: "foo".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            arguments: vec!["bar".into()],
            cwd: None,
            program: "echo".into(),
          }),
        }),
        Event::AgentToolResult {
          id: "foo".into(),
          result: tool_result,
        },
        Event::AgentDelta("done".into()),
        Event::AgentDone,
      ],
    );
  }
}
