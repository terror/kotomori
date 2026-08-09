use {
  super::*,
  ::rig::{completion::CompletionModel, streaming::StreamedAssistantContent},
};

#[derive(Clone)]
pub(super) struct Rig<M> {
  pub(super) model: M,
  pub(super) provider: &'static str,
}

impl<M> Debug for Rig<M> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Rig")
      .field("provider", &self.provider)
      .finish_non_exhaustive()
  }
}

#[async_trait]
impl<M> Provider for Rig<M>
where
  M: CompletionModel + 'static,
{
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    let request = CompletionRequest::from(&request);

    let mut stream = self.model.stream(request).await?;

    while let Some(chunk) = stream.next().await {
      match chunk? {
        StreamedAssistantContent::Text(text) if !text.text.is_empty() => {
          sink.delta(text.text)?;
        }
        StreamedAssistantContent::Reasoning(reasoning) => {
          sink.reasoning(reasoning)?;
        }
        StreamedAssistantContent::ReasoningDelta { id, reasoning }
          if !reasoning.is_empty() =>
        {
          sink.reasoning_delta(id, reasoning)?;
        }
        StreamedAssistantContent::ToolCall {
          internal_call_id,
          tool_call,
        } => {
          let id = if tool_call.id.is_empty() {
            internal_call_id
          } else {
            tool_call.id
          };

          sink.tool_call(RawToolCall {
            arguments: tool_call.function.arguments,
            id,
            name: tool_call.function.name,
          });
        }
        StreamedAssistantContent::Final(_)
        | StreamedAssistantContent::ReasoningDelta { .. }
        | StreamedAssistantContent::Text(_)
        | StreamedAssistantContent::ToolCallDelta { .. } => {}
      }
    }

    Ok(())
  }
}
