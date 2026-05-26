use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  content: String,
  event_sender: UnboundedSender<Event>,
  tool_calls: Vec<ToolInvocation>,
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

  pub(crate) fn new(event_sender: UnboundedSender<Event>) -> Self {
    Self {
      content: String::new(),
      event_sender,
      tool_calls: Vec::new(),
    }
  }

  pub(crate) fn tool_call(&mut self, tool_call: RawToolCall) -> Result {
    let tool_call = TryInto::<ToolInvocation>::try_into(tool_call)?;

    self.tool_calls.push(tool_call.clone());

    Ok(self.event_sender.send(Event::AgentToolCall(tool_call))?)
  }
}
