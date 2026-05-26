use super::*;

#[derive(Debug, Clone)]
pub(crate) struct ProviderSink {
  content: Arc<Mutex<String>>,
  event_sender: UnboundedSender<Event>,
  tool_calls: Arc<Mutex<Vec<ToolInvocation>>>,
}

impl ProviderSink {
  pub(crate) fn delta(&self, delta: impl Into<String>) -> Result {
    let delta = delta.into();

    self
      .content
      .lock()
      .expect("failed to lock provider content")
      .push_str(&delta);

    Ok(self.event_sender.send(Event::AgentDelta(delta))?)
  }

  pub(crate) fn finish(&self) -> ProviderOutput {
    ProviderOutput {
      content: self
        .content
        .lock()
        .expect("failed to lock provider content")
        .clone(),
      tool_calls: self
        .tool_calls
        .lock()
        .expect("failed to lock provider tool calls")
        .clone(),
    }
  }

  pub(crate) fn new(event_sender: UnboundedSender<Event>) -> Self {
    Self {
      content: Arc::new(Mutex::new(String::new())),
      event_sender,
      tool_calls: Arc::new(Mutex::new(Vec::new())),
    }
  }

  pub(crate) fn tool_call(&self, tool_call: RawToolCall) -> Result {
    let tool_call: ToolInvocation = tool_call.try_into()?;

    self
      .tool_calls
      .lock()
      .expect("failed to lock provider tool calls")
      .push(tool_call.clone());

    Ok(self.event_sender.send(Event::AgentToolCall(tool_call))?)
  }
}
