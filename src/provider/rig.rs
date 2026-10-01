use {
  super::*,
  ::rig::{
    DynModel,
    operation::Completion,
    streaming::{Item, StreamEvent},
  },
};

#[derive(Clone, Debug)]
pub(super) struct Rig {
  model: DynModel<Completion>,
}

impl Rig {
  pub(super) fn build(
    model: impl Into<DynModel<Completion>>,
  ) -> Arc<dyn Provider> {
    Arc::new(Self {
      model: model.into(),
    })
  }
}

#[async_trait]
impl Provider for Rig {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    let mut stream = self.model.stream(CompletionRequest::from(&request))?;

    let mut message_id = None;

    while let Some(item) = stream.next().await {
      let id = stream.message_id();

      if id != message_id {
        if let Some(id) = &id {
          sink.update(MessageUpdate::MessageId(id.clone()))?;
        }

        message_id = id;
      }

      let Item::Event(event) = item? else {
        continue;
      };

      let update = match event {
        StreamEvent::Text { part, text } => MessageUpdate::Text {
          delta: text,
          index: part.index(),
        },
        StreamEvent::Reasoning { part, text } => {
          MessageUpdate::ReasoningDelta {
            delta: text,
            index: part.index(),
          }
        }
        StreamEvent::End { part, content } => MessageUpdate::Content {
          content,
          index: part.index(),
        },
        StreamEvent::Start { .. } | StreamEvent::Arguments { .. } => continue,
      };

      sink.update(update)?;
    }

    let response = stream.finish().await?;

    sink.update(MessageUpdate::Complete(AgentMessage {
      content: response.choice,
      id: response.message_id,
    }))
  }
}
