use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  content: String,
  event_sender: UnboundedSender<Event>,
  reasoning_buffer: ReasoningBuffer,
  tool_calls: Vec<ToolInvocation>,
  tool_registry: ToolRegistry,
}

impl ProviderSink {
  pub(crate) fn delta(&mut self, delta: impl Into<String>) -> Result {
    let delta = delta.into();

    self.content.push_str(&delta);

    Ok(self.event_sender.send(Event::AgentDelta(delta))?)
  }

  pub(crate) fn finish(self) -> ProviderOutput {
    ProviderOutput {
      content: self.content,
      tool_calls: self.tool_calls,
    }
  }

  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    tool_registry: ToolRegistry,
  ) -> Self {
    Self {
      content: String::new(),
      event_sender,
      reasoning_buffer: ReasoningBuffer::default(),
      tool_calls: Vec::new(),
      tool_registry,
    }
  }

  pub(crate) fn reasoning(&mut self, reasoning: Reasoning) -> Result {
    if let Some(delta) = self.reasoning_buffer.push_reasoning(reasoning) {
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
      self.event_sender.send(Event::AgentReasoningDelta(delta))?;
    }

    Ok(())
  }

  pub(crate) fn tool_call(&mut self, tool_call: RawToolCall) -> Result {
    let tool_call = self.tool_registry.invocation(tool_call)?;

    self.tool_calls.push(tool_call.clone());

    Ok(self.event_sender.send(Event::AgentToolCall(tool_call))?)
  }
}
