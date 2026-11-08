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
    tool_call: &ToolInvocation,
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
        let _ = event_sender.send(AgentEvent::Error(error.to_string()));
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

      let message = self.provider.stream(request, &sink).await?;

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
          call: tool_call.protocol.id,
          name: tool_call.protocol.function.name,
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
