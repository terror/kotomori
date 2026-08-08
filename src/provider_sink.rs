use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  content: Vec<AgentMessageContent>,
  event_sender: UnboundedSender<Event>,
  reasoning_buffer: ReasoningBuffer,
  run_id: u64,
  tool_registry: ToolRegistry,
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

  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    run_id: u64,
    tool_registry: ToolRegistry,
  ) -> Self {
    Self {
      content: Vec::new(),
      event_sender,
      reasoning_buffer: ReasoningBuffer::default(),
      run_id,
      tool_registry,
    }
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
