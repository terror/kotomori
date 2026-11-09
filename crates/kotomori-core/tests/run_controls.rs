use {
  futures_util::future::BoxFuture,
  kotomori_core::{
    Agent, AgentConfig, AgentEvent, ApprovalPolicy, AssistantContent,
    Conversation, Message, MessageUpdate, Provider, ProviderResponse,
    ProviderSink, Request, Result, RunOutcome, Tool, ToolApproval, ToolCall,
    ToolDefinition, ToolName, ToolOutcome, ToolRegistry, ToolResult,
    UserMessageContent,
  },
  serde_json::{Value, json},
  std::sync::Arc,
};

#[derive(Debug)]
struct TestProvider;

impl Provider for TestProvider {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<ProviderResponse>> {
    Box::pin(async move {
      let message = if request.messages.is_empty() {
        vec![AssistantContent::tool_call(
          "foo",
          ToolName::new("foo")?,
          json!({}),
        )]
      } else {
        sink.update(MessageUpdate::Text {
          delta: "bar".into(),
          index: 0,
        })?;

        vec![AssistantContent::text("bar")]
      };

      Ok(ProviderResponse {
        message: message.into(),
        ..Default::default()
      })
    })
  }
}

#[derive(Debug)]
struct TestTool;

impl Tool for TestTool {
  fn definition(&self) -> Result<ToolDefinition> {
    Ok(ToolDefinition {
      description: "foo".into(),
      name: ToolName::new("foo")?,
      parameters: json!({"type": "object"}),
    })
  }

  fn prepare(&self, _: Value) -> Result<ToolCall> {
    Ok(ToolCall::new(
      ApprovalPolicy::Required,
      |context| async move {
        context.progress(json!("foo")).unwrap();

        ToolResult {
          content: "foo".into(),
          outcome: ToolOutcome::Success,
          ..Default::default()
        }
      },
    ))
  }
}

fn agent() -> Agent {
  let mut tools = ToolRegistry::default();

  tools.register(Arc::new(TestTool)).unwrap();

  Agent::new(
    AgentConfig {
      generation: Default::default(),
      max_rounds: 2,
      model: "foo:bar".parse().unwrap(),
      tools,
    },
    Arc::new(TestProvider),
    String::new(),
  )
}

#[tokio::test]
async fn approvals_are_controlled_without_owning_events() {
  for approval in [ToolApproval::Approved, ToolApproval::Denied] {
    let mut run = agent().spawn(Conversation::default());

    let control = run.control();

    while let Some(event) = run.recv().await {
      assert_eq!(
        serde_json::from_str::<AgentEvent>(
          &serde_json::to_string(&event).unwrap()
        )
        .unwrap(),
        event.clone(),
      );

      match event {
        AgentEvent::Message {
          message,
          response,
          run: id,
        } => {
          assert_eq!(id, run.id());
          assert_eq!(response.unwrap().run, id);
          assert_eq!(
            run.state().conversation.messages().last(),
            Some(&message)
          );
        }
        AgentEvent::ToolApprovalRequest(request) => {
          assert_eq!(request.id.response.run, run.id());

          let control = control.clone();

          tokio::spawn(async move {
            let mut stale = request.id.clone();

            stale.response.round += 1;

            assert!(!control.resolve_approval(&stale, approval));
            assert!(control.resolve_approval(&request.id, approval));
            assert!(!control.resolve_approval(&request.id, approval));
          })
          .await
          .unwrap();
        }
        AgentEvent::Update { response, update } => {
          assert_eq!(response.run, run.id());
          assert_eq!(
            update,
            MessageUpdate::Text {
              delta: "bar".into(),
              index: 0
            }
          );
          assert_eq!(run.state().message.message().text(), Some("bar".into()));
        }
        _ => {}
      }
    }

    let state = run.state().clone();

    let result = run.join().await;

    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(result.conversation, state.conversation);
    assert_eq!(result.entries, state.entries);
    assert_eq!(result.responses, state.responses);

    let outcome = if approval == ToolApproval::Approved {
      ToolOutcome::Success
    } else {
      ToolOutcome::Denied
    };

    let Message::User(content) = &result.conversation.messages()[1] else {
      panic!("missing tool result");
    };

    let UserMessageContent::ToolResult { result, .. } = &content[0] else {
      panic!("missing tool result");
    };

    assert_eq!(result.outcome, outcome);
  }
}

#[tokio::test]
async fn observed_approvals_are_resolved_when_joining() {
  for cancel in [false, true] {
    let mut run = agent().spawn(Conversation::default());

    let control = run.control();

    let request = loop {
      if let Some(AgentEvent::ToolApprovalRequest(request)) = run.recv().await {
        break request;
      }
    };

    if cancel {
      control.cancel();
    }

    let result = run.join().await;

    assert_eq!(
      result.outcome,
      if cancel {
        RunOutcome::Cancelled
      } else {
        RunOutcome::Completed
      },
    );

    result.conversation.validate().unwrap();

    assert!(!control.resolve_approval(&request.id, ToolApproval::Approved));
  }
}
