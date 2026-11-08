use super::*;

#[derive(Debug)]
pub struct Agent {
  config: AgentConfig,
  provider: Arc<dyn Provider>,
  system: String,
  task: Option<task::JoinHandle<()>>,
}

impl Agent {
  async fn execute_tool(
    &self,
    run: &mut RunContext,
    conversation: &mut Conversation,
    tool_id: ToolId,
    invocation: ToolInvocation,
    tool: ToolCall,
  ) -> Result {
    let approval = if self.config.yolo || tool.approval == ApprovalPolicy::None
    {
      ToolApproval::Approved
    } else {
      run
        .approval(ApprovalRequest {
          invocation: invocation.clone(),
          run_id: run.run_id,
          tool_id,
        })
        .await?
    };

    let result = match approval {
      ToolApproval::Approved => {
        run.send(AgentEvent::ToolStarted { tool_id })?;
        tool.future.await
      }
      ToolApproval::Denied => ToolResult {
        stderr: Some("permission denied".into()),
        ..Default::default()
      },
    };

    let message_id = run.commit(
      conversation,
      Message::User(vec![UserMessageContent::ToolResult {
        call: invocation.protocol.id,
        name: invocation.protocol.function.name,
        result,
      }]),
      None,
      None,
    )?;

    run.send(match approval {
      ToolApproval::Approved => AgentEvent::ToolCompleted {
        tool_id,
        message_id,
      },
      ToolApproval::Denied => AgentEvent::ToolDenied {
        tool_id,
        message_id,
      },
    })
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
  ) -> (RunControl, UnboundedReceiver<RunEvent>) {
    self.interrupt();

    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    let control = RunControl::new();

    let mut run = RunContext::new(&control, event_sender);

    let agent = Self {
      config: self.config.clone(),
      provider: self.provider.clone(),
      system: self.system.clone(),
      task: None,
    };

    self.task = Some(tokio::spawn(async move {
      let result = async {
        run.send(AgentEvent::RunStarted)?;
        agent.stream(&mut run, conversation).await
      }
      .await;

      let _ = run.send(match result {
        Ok(()) => AgentEvent::RunCompleted,
        Err(error) => AgentEvent::RunFailed(error.to_string()),
      });
    }));

    (control, event_receiver)
  }

  async fn stream(
    &self,
    run: &mut RunContext,
    mut conversation: Conversation,
  ) -> Result {
    let tools = self.config.tools.definitions();

    for message in conversation.interrupt_pending_calls() {
      let message_id = run.message_id()?;

      run.send(AgentEvent::MessageCommitted {
        draft: None,
        message_id,
        message,
      })?;
    }

    loop {
      let message_id = run.message_id()?;

      let mut sink =
        ProviderSink::new(run.event_sender.clone(), run.run_id, message_id);

      let request = Request {
        messages: conversation.messages(),
        model: &self.config.model,
        system: (!self.system.is_empty()).then_some(self.system.as_str()),
        tools: &tools,
      };

      let message = self.provider.stream(request, &mut sink).await;

      let draft = sink.finish();

      let message = match message {
        Ok(message) => message,
        Err(error) => {
          run.settle(&mut conversation, message_id, draft)?;
          return Err(error);
        }
      };

      if message.content.is_empty() {
        run.settle(&mut conversation, message_id, draft)?;
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
        .collect::<Result<Vec<_>>>();

      let tool_calls = match tool_calls {
        Ok(tool_calls) => tool_calls,
        Err(error) => {
          run.settle(&mut conversation, message_id, draft)?;
          return Err(error);
        }
      };

      run.commit(
        &mut conversation,
        Message::Agent(message),
        Some(message_id),
        None,
      )?;

      if tool_calls.is_empty() {
        break;
      }

      let tool_calls = tool_calls
        .into_iter()
        .map(|(invocation, tool)| {
          let tool_id = run.tool_id()?;

          run.send(AgentEvent::ToolRequested {
            tool_id,
            message_id,
            invocation: invocation.clone(),
          })?;

          Ok((tool_id, invocation, tool))
        })
        .collect::<Result<Vec<_>>>()?;

      for (tool_id, invocation, tool) in tool_calls {
        self
          .execute_tool(run, &mut conversation, tool_id, invocation, tool)
          .await?;
      }
    }

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
      sink: &'a mut ProviderSink,
    ) -> BoxFuture<'a, Result<AgentMessage>> {
      Box::pin(async move {
        assert_eq!(request.tools, [TestTool.definition()]);

        self
          .requests
          .lock()
          .unwrap()
          .push(request.messages.to_vec());

        let response = self.responses.lock().unwrap().pop_front().unwrap();

        sink.update_many(response.updates)?;

        if let Some(error) = response.error {
          bail!(error);
        }

        Ok(response.message)
      })
    }
  }

  #[derive(Debug, Default)]
  struct TestResponse {
    error: Option<&'static str>,
    message: AgentMessage,
    updates: Vec<MessageUpdate>,
  }

  #[derive(Debug)]
  struct TestTool;

  impl Tool for TestTool {
    fn definition(&self) -> ToolDefinition {
      ToolDefinition {
        description: "bar".into(),
        name: "foo".into(),
        parameters: json!({"type": "string"}),
      }
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
    async fn assert_events(
      events: &mut UnboundedReceiver<RunEvent>,
      run_id: RunId,
      expected: impl IntoIterator<Item = AgentEvent>,
    ) {
      for event in expected {
        let event = RunEvent { event, run_id };

        assert_eq!(events.recv().await.unwrap(), event);
        assert_eq!(
          serde_json::from_str::<RunEvent>(
            &serde_json::to_string(&event).unwrap()
          )
          .unwrap(),
          event,
        );
      }
    }

    fn assert_requests(&self, expected: &[Vec<Message>]) {
      let requests = self.requests.lock().unwrap();

      assert_eq!(
        serde_json::from_str::<Vec<Vec<Message>>>(
          &serde_json::to_string(&*requests).unwrap(),
        )
        .unwrap(),
        expected,
      );
    }

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

  #[allow(clippy::too_many_lines)]
  #[tokio::test]
  async fn approvals_can_be_denied() {
    for respond in [true, false] {
      let invocation = ToolInvocation {
        protocol: ToolRequest::new(
          CallId::new("foo"),
          ToolFunction {
            arguments: json!("bar"),
            name: ToolName::new("foo").unwrap(),
          },
        ),
      };

      let tool_call = AssistantContent::ToolCall(invocation.protocol.clone());

      let mut test_agent = TestAgent::new(
        vec![
          TestResponse {
            message: vec![tool_call.clone()].into(),
            ..Default::default()
          },
          TestResponse::default(),
        ],
        false,
      );

      let user = Message::User(vec![UserMessageContent::Text("foo".into())]);

      let (control, mut events) =
        test_agent.agent.spawn(vec![user.clone()].into());

      let run_id = control.run_id();

      TestAgent::assert_events(
        &mut events,
        run_id,
        [
          AgentEvent::RunStarted,
          AgentEvent::MessageCommitted {
            draft: None,
            message_id: MessageId(0),
            message: Message::Agent(vec![tool_call.clone()].into()),
          },
          AgentEvent::ToolRequested {
            tool_id: ToolId(0),
            message_id: MessageId(0),
            invocation: invocation.clone(),
          },
          AgentEvent::ApprovalRequested(ApprovalRequest {
            invocation,
            run_id,
            tool_id: ToolId(0),
          }),
        ],
      )
      .await;

      let response = ApprovalResponse {
        approval: ToolApproval::Denied,
        run_id,
        tool_id: ToolId(0),
      };

      assert!(!control.respond(ApprovalResponse {
        run_id: RunId(run_id.0 + 1),
        ..response
      }));
      assert!(!control.respond(ApprovalResponse {
        tool_id: ToolId(1),
        ..response
      }));
      if respond {
        assert!(control.respond(response));
        assert!(!control.respond(response));
      } else {
        drop(control);
      }

      let tool_result = Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::new("foo"),
        name: ToolName::new("foo").unwrap(),
        result: ToolResult {
          stderr: Some("permission denied".into()),
          ..Default::default()
        },
      }]);

      TestAgent::assert_events(
        &mut events,
        run_id,
        [
          AgentEvent::ApprovalResolved {
            tool_id: ToolId(0),
            approval: ToolApproval::Denied,
          },
          AgentEvent::MessageCommitted {
            draft: None,
            message_id: MessageId(1),
            message: tool_result.clone(),
          },
          AgentEvent::ToolDenied {
            tool_id: ToolId(0),
            message_id: MessageId(1),
          },
          AgentEvent::MessagePreviewFinished {
            draft: None,
            message_id: MessageId(2),
          },
          AgentEvent::RunCompleted,
        ],
      )
      .await;

      assert_eq!(events.recv().await, None);

      test_agent.assert_requests(&[
        vec![user.clone()],
        vec![user, Message::Agent(vec![tool_call].into()), tool_result],
      ]);
    }
  }

  #[allow(clippy::too_many_lines)]
  #[tokio::test]
  async fn loops_until_provider_returns_no_tool_calls() {
    async fn case(
      invocation: &ToolInvocation,
      message: &AgentMessage,
      response: TestResponse,
      expected: Vec<AgentEvent>,
    ) {
      let update = MessageUpdate::Text {
        delta: "quux".into(),
        index: 0,
      };

      let mut test_agent = TestAgent::new(
        vec![
          TestResponse {
            message: message.clone(),
            updates: vec![update.clone()],
            ..Default::default()
          },
          response,
        ],
        true,
      );

      let user = Message::User(vec![UserMessageContent::Text("foo".into())]);
      let (control, mut events) =
        test_agent.agent.spawn(vec![user.clone()].into());
      let tool_result = Message::User(vec![UserMessageContent::ToolResult {
        call: invocation.protocol.id.clone(),
        name: invocation.protocol.function.name.clone(),
        result: ToolResult {
          content: Some("bar".into()),
          outcome: ToolOutcome::Success,
          ..Default::default()
        },
      }]);

      TestAgent::assert_events(
        &mut events,
        control.run_id(),
        [
          AgentEvent::RunStarted,
          AgentEvent::MessagePreview {
            message_id: MessageId(0),
            update,
          },
          AgentEvent::MessageCommitted {
            draft: None,
            message_id: MessageId(0),
            message: Message::Agent(message.clone()),
          },
          AgentEvent::ToolRequested {
            tool_id: ToolId(0),
            message_id: MessageId(0),
            invocation: invocation.clone(),
          },
          AgentEvent::ToolStarted { tool_id: ToolId(0) },
          AgentEvent::MessageCommitted {
            draft: None,
            message_id: MessageId(1),
            message: tool_result.clone(),
          },
          AgentEvent::ToolCompleted {
            tool_id: ToolId(0),
            message_id: MessageId(1),
          },
        ]
        .into_iter()
        .chain(expected),
      )
      .await;

      assert_eq!(events.recv().await, None);
      test_agent.assert_requests(&[
        vec![user.clone()],
        vec![user, Message::Agent(message.clone()), tool_result],
      ]);
    }

    let reasoning = Reasoning {
      content: vec![ReasoningContent::Encrypted("foo".into())],
      id: Some("bar".into()),
      issuer: "mock".into(),
    };

    let invocation = ToolInvocation {
      protocol: ToolRequest {
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
      },
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
        AssistantContent::ToolCall(invocation.protocol.clone()),
        AssistantContent::text("baz"),
      ],
      id: Some("quuz".into()),
    };

    let complete = AgentMessage::from(vec![AssistantContent::text("bar")]);
    let update = MessageUpdate::Text {
      delta: "bar".into(),
      index: 0,
    };

    case(
      &invocation,
      &message,
      TestResponse {
        message: complete.clone(),
        updates: vec![update.clone()],
        ..Default::default()
      },
      vec![
        AgentEvent::MessagePreview {
          message_id: MessageId(2),
          update,
        },
        AgentEvent::MessageCommitted {
          draft: None,
          message_id: MessageId(2),
          message: Message::Agent(complete),
        },
        AgentEvent::RunCompleted,
      ],
    )
    .await;

    let partial = AgentMessage {
      content: vec![
        AssistantContent::reasoning("foo", "bar"),
        AssistantContent::text("baz"),
      ],
      id: Some("qux".into()),
    };
    let updates = vec![
      MessageUpdate::Complete(partial.clone()),
      MessageUpdate::ReasoningDelta {
        delta: "quux".into(),
        index: 2,
      },
    ];
    let mut draft = MessageBuffer::default();
    draft.apply_many(&updates);

    let invalid = AssistantContent::tool_call(
      "foo",
      ToolName::new("foo").unwrap(),
      json!(false),
    );

    for (response, terminal) in [
      (
        TestResponse {
          error: Some("foo"),
          ..Default::default()
        },
        AgentEvent::RunFailed("foo".into()),
      ),
      (TestResponse::default(), AgentEvent::RunCompleted),
      (
        TestResponse {
          message: vec![invalid.clone()].into(),
          ..Default::default()
        },
        AgentEvent::RunFailed("failed to decode `foo` arguments".into()),
      ),
    ] {
      case(
        &invocation,
        &message,
        TestResponse {
          updates: updates
            .iter()
            .cloned()
            .chain([MessageUpdate::Content {
              index: 3,
              content: invalid.clone(),
            }])
            .collect(),
          ..response
        },
        updates
          .iter()
          .cloned()
          .map(|update| AgentEvent::MessagePreview {
            message_id: MessageId(2),
            update,
          })
          .chain([
            AgentEvent::MessageCommitted {
              draft: Some(draft.clone()),
              message_id: MessageId(2),
              message: Message::Agent(partial.clone()),
            },
            terminal,
          ])
          .collect(),
      )
      .await;
    }

    let update = MessageUpdate::ReasoningDelta {
      delta: "foo".into(),
      index: 0,
    };
    let mut draft = MessageBuffer::default();
    draft.apply(update.clone());

    case(
      &invocation,
      &message,
      TestResponse {
        error: Some("bar"),
        updates: vec![update.clone()],
        ..Default::default()
      },
      vec![
        AgentEvent::MessagePreview {
          message_id: MessageId(2),
          update,
        },
        AgentEvent::MessagePreviewFinished {
          message_id: MessageId(2),
          draft: Some(draft),
        },
        AgentEvent::RunFailed("bar".into()),
      ],
    )
    .await;

    for error in [None, Some("foo")] {
      case(
        &invocation,
        &message,
        TestResponse {
          error,
          ..Default::default()
        },
        vec![
          AgentEvent::MessagePreviewFinished {
            message_id: MessageId(2),
            draft: None,
          },
          error.map_or(AgentEvent::RunCompleted, |error| {
            AgentEvent::RunFailed(error.into())
          }),
        ],
      )
      .await;
    }
  }
}
