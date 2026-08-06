use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  content: Vec<AgentMessageContent>,
  event_sender: UnboundedSender<Event>,
  reasoning_buffer: ReasoningBuffer,
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

    Ok(self.event_sender.send(Event::AgentDelta(delta))?)
  }

  pub(crate) fn finish(self) -> Vec<AgentMessageContent> {
    self.content
  }

  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    tool_registry: ToolRegistry,
  ) -> Self {
    Self {
      content: Vec::new(),
      event_sender,
      reasoning_buffer: ReasoningBuffer::default(),
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
      self.event_sender.send(Event::AgentReasoningDelta(delta))?;
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
      self.event_sender.send(Event::AgentReasoningDelta(delta))?;
    }

    Ok(())
  }

  pub(crate) fn tool_call(&mut self, tool_call: RawToolCall) -> Result {
    let tool_call = self.tool_registry.invocation(tool_call)?;

    self
      .content
      .push(AgentMessageContent::ToolCall(tool_call.clone()));

    Ok(self.event_sender.send(Event::AgentToolCall(tool_call))?)
  }
}
