use {
  futures_util::future::BoxFuture,
  kotomori_core::{
    Agent, AgentConfig, AgentMessage, ApprovalPolicy, AssistantContent, CallId,
    ContextStrategy, Conversation, HostPolicy, Message, PolicyDecision,
    Provider, ProviderResponse, ProviderSink, Request, RequestContext, Result,
    RunContext, RunId, RunLimit, RunOutcome, Tool, ToolCall, ToolDefinition,
    ToolName, ToolOutcome, ToolRegistry, ToolRequest, ToolResult,
    UserMessageContent,
  },
  serde_json::{Value, json},
  std::{
    sync::{
      Arc, Mutex,
      atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
  },
  tokio::{sync::Notify, time::timeout},
};

#[derive(Debug, Default)]
struct TestExtension {
  contexts: Mutex<Vec<(RunId, Option<Instant>, bool)>>,
  entered: Notify,
  error: Option<&'static str>,
  evaluations: AtomicU64,
  executions: Arc<AtomicU64>,
  pending: bool,
}

impl TestExtension {
  fn record(&self, context: &RunContext) {
    self.contexts.lock().unwrap().push((
      context.id(),
      context.deadline(),
      context.is_cancelled(),
    ));
  }
}

impl ContextStrategy for TestExtension {
  fn prepare<'a>(
    &'a self,
    context: &'a RunContext,
    request: Request<'a>,
  ) -> BoxFuture<'a, Result<RequestContext<'a>>> {
    Box::pin(async move {
      self.record(context);
      Ok(request.into())
    })
  }
}

impl HostPolicy for TestExtension {
  fn evaluate<'a>(
    &'a self,
    context: &'a RunContext,
    _invocation: &'a ToolRequest,
    _approval: ApprovalPolicy,
  ) -> BoxFuture<'a, Result<PolicyDecision>> {
    Box::pin(async move {
      self.record(context);
      self.evaluations.fetch_add(1, Ordering::Relaxed);
      self.entered.notify_one();

      if self.pending {
        context.cancelled().await;
      }

      if let Some(error) = self.error {
        anyhow::bail!(error);
      }

      Ok(if self.executions.load(Ordering::Relaxed) == 0 {
        PolicyDecision::Allow
      } else {
        PolicyDecision::Deny {
          reason: "bar".into(),
        }
      })
    })
  }
}

impl Tool for TestExtension {
  fn definition(&self) -> Result<ToolDefinition> {
    Ok(ToolDefinition {
      description: "foo".into(),
      name: ToolName::new("foo")?,
      parameters: json!({"type": "string"}),
    })
  }

  fn prepare(&self, arguments: Value) -> Result<ToolCall> {
    let content = serde_json::from_value::<String>(arguments)?;
    let executions = self.executions.clone();

    Ok(ToolCall::new(ApprovalPolicy::None, |_| async move {
      executions.fetch_add(1, Ordering::Relaxed);

      ToolResult {
        content,
        outcome: ToolOutcome::Success,
        ..Default::default()
      }
    }))
  }
}

#[derive(Debug)]
struct TestProvider(AgentMessage);

impl Provider for TestProvider {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    _sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<ProviderResponse>> {
    Box::pin(async move {
      Ok(ProviderResponse {
        message: if request.messages.is_empty() {
          self.0.clone()
        } else {
          AgentMessage::default()
        },
        ..Default::default()
      })
    })
  }
}

fn agent(
  extension: &Arc<TestExtension>,
  arguments: [Value; 2],
) -> (Agent, Message) {
  let mut tools = ToolRegistry::default();

  tools.register(extension.clone()).unwrap();

  let message = AgentMessage::from(
    ["foo", "bar"]
      .into_iter()
      .zip(arguments)
      .map(|(id, arguments)| {
        AssistantContent::tool_call(
          id,
          ToolName::new("foo").unwrap(),
          arguments,
        )
      })
      .collect::<Vec<_>>(),
  );

  (
    Agent::new(
      AgentConfig {
        generation: kotomori_core::GenerationOptions::default(),
        max_rounds: 2,
        model: "foo:bar".parse().unwrap(),
        tools,
      },
      Arc::new(TestProvider(message.clone())),
      String::new(),
    )
    .with_context_strategy(extension.clone())
    .with_host_policy(extension.clone()),
    Message::Agent(message),
  )
}

#[tokio::test]
async fn policies_observe_previous_tool_execution_and_preserve_denial_reason() {
  let extension = Arc::new(TestExtension::default());
  let (agent, message) = agent(&extension, [json!("foo"), json!("bar")]);
  let deadline = Instant::now() + Duration::from_secs(5);
  let run = agent.spawn_with_deadline(Conversation::default(), deadline);
  let id = run.id();
  let result = run.join().await;

  assert_eq!(result.outcome, RunOutcome::Completed);
  assert_eq!(
    result.conversation.messages(),
    [
      message,
      Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::new("foo"),
        name: ToolName::new("foo").unwrap(),
        result: ToolResult {
          content: "foo".into(),
          outcome: ToolOutcome::Success,
          ..Default::default()
        },
      }]),
      Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::new("bar"),
        name: ToolName::new("foo").unwrap(),
        result: ToolResult {
          content: "bar".into(),
          outcome: ToolOutcome::Denied,
          ..Default::default()
        },
      }]),
    ]
  );
  assert_eq!(extension.evaluations.load(Ordering::Relaxed), 2);
  assert_eq!(extension.executions.load(Ordering::Relaxed), 1);
  assert_eq!(
    *extension.contexts.lock().unwrap(),
    [(id, Some(deadline), false); 4]
  );
}

#[tokio::test]
async fn invalid_batches_skip_policies_and_policy_errors_stop_execution() {
  async fn case(arguments: [Value; 2], outcome: RunOutcome, evaluations: u64) {
    let extension = Arc::new(TestExtension {
      error: Some("foo"),
      ..Default::default()
    });
    let (agent, _) = agent(&extension, arguments);
    let result = agent.spawn(Conversation::default()).join().await;

    assert_eq!(result.outcome, outcome);
    assert_eq!(extension.evaluations.load(Ordering::Relaxed), evaluations);
    assert_eq!(extension.executions.load(Ordering::Relaxed), 0);
    result.conversation.validate().unwrap();
  }

  case([json!("foo"), json!({})], RunOutcome::Completed, 0).await;
  case(
    [json!("foo"), json!("bar")],
    RunOutcome::Failed("foo".into()),
    1,
  )
  .await;
}

#[tokio::test]
async fn pending_policies_stop_on_cancellation_and_deadlines() {
  async fn case(deadline: Option<Instant>, outcome: RunOutcome) {
    let extension = Arc::new(TestExtension {
      pending: true,
      ..Default::default()
    });
    let (agent, _) = agent(&extension, [json!("foo"), json!("bar")]);
    let run = if let Some(deadline) = deadline {
      agent.spawn_with_deadline(Conversation::default(), deadline)
    } else {
      agent.spawn(Conversation::default())
    };

    timeout(Duration::from_secs(5), extension.entered.notified())
      .await
      .unwrap();

    if deadline.is_none() {
      run.cancel();
    }

    let result = timeout(Duration::from_secs(5), run.join()).await.unwrap();

    assert_eq!(result.outcome, outcome);
    assert_eq!(extension.executions.load(Ordering::Relaxed), 0);
    result.conversation.validate().unwrap();
  }

  case(None, RunOutcome::Cancelled).await;
  case(
    Some(Instant::now() + Duration::from_millis(100)),
    RunOutcome::Limited(RunLimit::Deadline),
  )
  .await;
}
