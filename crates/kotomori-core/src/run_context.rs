use super::*;

pub(crate) struct RunContext {
  pub(crate) event_sender: UnboundedSender<RunEvent>,
  next_message_id: u64,
  next_tool_id: u64,
  pending: Weak<Mutex<Option<PendingApproval>>>,
  pub(crate) run_id: RunId,
}

impl RunContext {
  pub(crate) async fn approval(
    &self,
    request: ApprovalRequest,
  ) -> Result<ToolApproval> {
    let (response_sender, response_receiver) = oneshot::channel();

    if let Some(pending) = self.pending.upgrade() {
      *pending.lock().unwrap() = Some(PendingApproval {
        response_sender,
        tool_id: request.tool_id,
      });
    } else {
      drop(response_sender);
    }

    let tool_id = request.tool_id;

    self.send(AgentEvent::ApprovalRequested(request))?;

    let approval = response_receiver.await.unwrap_or(ToolApproval::Denied);

    self.send(AgentEvent::ApprovalResolved { tool_id, approval })?;

    Ok(approval)
  }

  pub(crate) fn commit(
    &mut self,
    conversation: &mut Conversation,
    message: Message,
    message_id: Option<MessageId>,
  ) -> Result<MessageId> {
    for message in conversation.push(message.clone()) {
      let message_id = self.message_id()?;

      self.send(AgentEvent::MessageCommitted {
        message_id,
        message,
      })?;
    }

    let message_id = match message_id {
      Some(message_id) => message_id,
      None => self.message_id()?,
    };

    self.send(AgentEvent::MessageCommitted {
      message_id,
      message,
    })?;

    Ok(message_id)
  }

  pub(crate) fn message_id(&mut self) -> Result<MessageId> {
    let message_id = MessageId(self.next_message_id);

    self.next_message_id = self
      .next_message_id
      .checked_add(1)
      .context("message ID overflow")?;

    Ok(message_id)
  }

  pub(crate) fn new(
    control: &RunControl,
    event_sender: UnboundedSender<RunEvent>,
  ) -> Self {
    Self {
      event_sender,
      next_message_id: 0,
      next_tool_id: 0,
      pending: Arc::downgrade(&control.pending),
      run_id: control.run_id(),
    }
  }

  pub(crate) fn send(&self, event: AgentEvent) -> Result {
    self.event_sender.send(RunEvent {
      event,
      run_id: self.run_id,
    })?;

    Ok(())
  }

  pub(crate) fn tool_id(&mut self) -> Result<ToolId> {
    let tool_id = ToolId(self.next_tool_id);

    self.next_tool_id = self
      .next_tool_id
      .checked_add(1)
      .context("tool ID overflow")?;

    Ok(tool_id)
  }
}
