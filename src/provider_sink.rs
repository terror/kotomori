use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  pub(super) event_sender: UnboundedSender<Event>,
  pub(super) message: MessageBuffer,
  pub(super) run_id: u64,
}

impl ProviderSink {
  pub(crate) fn delta(&mut self, delta: impl Into<String>) -> Result {
    self.update(MessageUpdate::Text(delta.into()))
  }

  pub(crate) fn finish(self) -> AgentMessage {
    self.message.finish()
  }

  pub(crate) fn message_id(&mut self, id: String) -> Result {
    self.update(MessageUpdate::MessageId(id))
  }

  pub(crate) fn reasoning(&mut self, reasoning: Reasoning) -> Result {
    self.update(MessageUpdate::Reasoning(reasoning))
  }

  pub(crate) fn reasoning_append(&mut self, reasoning: Reasoning) -> Result {
    self.update(MessageUpdate::ReasoningAppend(reasoning))
  }

  pub(crate) fn reasoning_delta(
    &mut self,
    id: Option<String>,
    delta: impl Into<String>,
  ) -> Result {
    self.update(MessageUpdate::ReasoningDelta {
      delta: delta.into(),
      id,
    })
  }

  pub(crate) fn tool_call(&mut self, tool_call: ::rig::message::ToolCall) {
    self.message.apply(MessageUpdate::ToolCall(tool_call));
  }

  fn update(&mut self, update: MessageUpdate) -> Result {
    self.message.apply(update.clone());

    Ok(self.event_sender.send(Event::Agent {
      event: AgentEvent::Update(update),
      run_id: self.run_id,
    })?)
  }
}

impl Default for ProviderSink {
  fn default() -> Self {
    let (sender, _) = mpsc::unbounded_channel();

    Self {
      event_sender: sender,
      message: MessageBuffer::default(),
      run_id: 0,
    }
  }
}
