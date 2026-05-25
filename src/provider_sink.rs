use super::*;

#[derive(Debug, Clone)]
pub(crate) struct ProviderSink {
  event_sender: UnboundedSender<Event>,
}

impl ProviderSink {
  pub(crate) fn delta(&self, delta: impl Into<String>) -> Result {
    Ok(self.event_sender.send(Event::AgentDelta(delta.into()))?)
  }

  pub(crate) fn new(event_sender: UnboundedSender<Event>) -> Self {
    Self { event_sender }
  }

  pub(crate) fn tool_call(&self, tool_call: RawToolCall) -> Result {
    Ok(self.event_sender.send(Event::AgentToolCall(tool_call))?)
  }
}
