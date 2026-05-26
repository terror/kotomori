use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Agent {
  event_sender: UnboundedSender<Event>,
  model: Model,
  provider: Arc<dyn Provider>,
}

impl Agent {
  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    model: Model,
  ) -> Result<Self> {
    let provider = Arc::<dyn Provider>::try_from(model.clone())?;

    Ok(Self {
      event_sender,
      model,
      provider,
    })
  }

  pub(crate) fn spawn(&self, messages: Vec<Message>) {
    let agent = self.clone();

    tokio::spawn(async move {
      if let Err(error) = agent.stream(messages).await {
        let _ = agent.event_sender.send(Event::Error(error.to_string()));
      }
    });
  }

  async fn stream(&self, mut messages: Vec<Message>) -> Result {
    loop {
      let sink = ProviderSink::new(self.event_sender.clone());

      self
        .provider
        .stream(
          Request::new(self.model.clone(), messages.clone()),
          sink.clone(),
        )
        .await?;

      let output = sink.finish();

      if !output.content.is_empty() {
        messages.push(Message::new(Role::Agent, output.content));
      }

      if output.tool_calls.is_empty() {
        break;
      }

      for tool_call in output.tool_calls {
        messages.push(tool_call.message());

        let result = tool_call.kind.execute();
        let content = result.message_content();
        let is_error = result.is_error();

        messages.push(Message::tool_result(
          tool_call.id.clone(),
          content,
          is_error,
        ));

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
    async fn stream(&self, request: Request, sink: ProviderSink) -> Result {
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
  async fn loops_until_provider_returns_no_tool_calls() {
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
    let requests = Arc::new(Mutex::new(Vec::new()));

    let agent = Agent {
      event_sender,
      model: "fake:local".parse().unwrap(),
      provider: Arc::new(LoopProvider {
        requests: requests.clone(),
      }),
    };

    agent
      .stream(vec![Message::new(Role::User, "foo")])
      .await
      .unwrap();

    let requests = requests.lock().unwrap();

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], [Message::new(Role::User, "foo")]);
    assert_eq!(requests[1].len(), 3);
    assert_eq!(requests[1][0], Message::new(Role::User, "foo"));

    assert!(matches!(
      requests[1][1].kind(),
      MessageKind::ToolUse { name, .. } if name == "command"
    ));

    assert!(matches!(
      requests[1][2].kind(),
      MessageKind::ToolResult {
        content,
        is_error: false,
        ..
      } if content.contains(r#""stdout":"bar\n""#)
    ));

    let mut events = Vec::new();

    while let Ok(event) = event_receiver.try_recv() {
      events.push(event);
    }

    assert!(matches!(events[0], Event::AgentToolCall(_)));
    assert!(matches!(events[1], Event::AgentToolResult { .. }));
    assert_eq!(events[2], Event::AgentDelta("done".into()));
    assert_eq!(events[3], Event::AgentDone);
  }
}
