use super::*;

#[derive(Debug)]
pub(crate) struct ProviderSink {
  content: String,
  event_sender: UnboundedSender<Event>,
  reasoning: String,
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
      reasoning: self.reasoning,
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
      reasoning: String::new(),
      tool_calls: Vec::new(),
      tool_registry,
    }
  }

  pub(crate) fn reasoning_delta(&mut self, delta: impl Into<String>) -> Result {
    let delta = delta.into();

    self.reasoning.push_str(&delta);

    Ok(self.event_sender.send(Event::AgentReasoningDelta(delta))?)
  }

  pub(crate) fn tool_call(&mut self, tool_call: RawToolCall) -> Result {
    let tool_call = self.tool_registry.invocation(tool_call)?;

    self.tool_calls.push(tool_call.clone());

    Ok(self.event_sender.send(Event::AgentToolCall(tool_call))?)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reasoning_deltas_are_collected_and_sent() {
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();

    let mut sink = ProviderSink::new(event_sender, ToolRegistry::default());

    sink.reasoning_delta("foo").unwrap();
    sink.reasoning_delta("bar").unwrap();

    assert_eq!(
      sink.finish(),
      ProviderOutput {
        content: String::new(),
        reasoning: "foobar".into(),
        tool_calls: Vec::new(),
      },
    );

    assert_eq!(
      event_receiver.try_recv().unwrap(),
      Event::AgentReasoningDelta("foo".into()),
    );

    assert_eq!(
      event_receiver.try_recv().unwrap(),
      Event::AgentReasoningDelta("bar".into()),
    );
  }
}
