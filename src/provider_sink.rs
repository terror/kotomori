use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  pub(super) content: Vec<AgentMessageContent>,
  pub(super) event_sender: UnboundedSender<Event>,
  pub(super) reasoning_buffer: ReasoningBuffer,
  pub(super) run_id: u64,
  pub(super) tool_registry: ToolRegistry,
}

impl ProviderSink {
  pub(crate) fn delta(&mut self, delta: impl Into<String>) -> Result {
    let delta = delta.into();

    if !delta.is_empty() {
      match self.content.last_mut() {
        Some(AgentMessageContent::Text(text)) => text.push_str(&delta),
        Some(
          AgentMessageContent::Reasoning(_) | AgentMessageContent::ToolCall(_),
        )
        | None => {
          self.content.push(AgentMessageContent::Text(delta.clone()));
        }
      }
    }

    Ok(self.event_sender.send(Event::Agent {
      event: AgentEvent::Delta(delta),
      run_id: self.run_id,
    })?)
  }

  pub(crate) fn finish(self) -> Vec<AgentMessageContent> {
    self.content
  }

  fn push_reasoning_delta(&mut self, delta: &str) {
    if delta.is_empty() {
      return;
    }

    match self.content.last_mut() {
      Some(AgentMessageContent::Reasoning(reasoning)) => {
        reasoning.push_str(delta);
      }
      Some(AgentMessageContent::Text(_) | AgentMessageContent::ToolCall(_))
      | None => {
        self
          .content
          .push(AgentMessageContent::Reasoning(delta.to_owned()));
      }
    }
  }

  pub(crate) fn reasoning(&mut self, reasoning: Reasoning) -> Result {
    if let Some(delta) = self.reasoning_buffer.push_reasoning(reasoning) {
      self.push_reasoning_delta(&delta);

      self.event_sender.send(Event::Agent {
        event: AgentEvent::ReasoningDelta(delta),
        run_id: self.run_id,
      })?;
    }

    Ok(())
  }

  pub(crate) fn reasoning_delta(
    &mut self,
    id: Option<String>,
    delta: impl Into<String>,
  ) -> Result {
    if let Some(delta) = self.reasoning_buffer.push_delta(id, delta) {
      self.push_reasoning_delta(&delta);

      self.event_sender.send(Event::Agent {
        event: AgentEvent::ReasoningDelta(delta),
        run_id: self.run_id,
      })?;
    }

    Ok(())
  }

  pub(crate) fn tool_call(&mut self, tool_call: RawToolCall) -> Result {
    let tool_call = self.tool_registry.invocation(tool_call)?;

    self
      .content
      .push(AgentMessageContent::ToolCall(tool_call.clone()));

    Ok(self.event_sender.send(Event::Agent {
      event: AgentEvent::ToolCall(tool_call),
      run_id: self.run_id,
    })?)
  }
}

impl Default for ProviderSink {
  fn default() -> Self {
    let (sender, _) = mpsc::unbounded_channel();

    Self {
      content: Vec::new(),
      event_sender: sender,
      reasoning_buffer: ReasoningBuffer::default(),
      run_id: 0,
      tool_registry: ToolRegistry::default(),
    }
  }
}
