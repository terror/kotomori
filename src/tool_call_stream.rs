use super::*;

pub(crate) struct ToolCallStream<I> {
  /// In-flight calls keyed by the provider's stream index.
  ///
  /// Multiple tool calls may be interleaved on the same response stream. A
  /// stable map preserves deterministic completion order for any calls that
  /// have to be flushed at end of stream.
  calls: BTreeMap<I, ToolCallBuilder>,
}

impl<I: Ord> ToolCallStream<I> {
  pub(crate) fn finish_all(self) -> Result<Vec<RawToolCall>> {
    self
      .calls
      .into_values()
      .map(ToolCallBuilder::finish)
      .collect()
  }

  fn push(&mut self, update: ToolCallUpdate<I>) -> Result<Vec<RawToolCall>> {
    match update.kind {
      ToolCallUpdateKind::ArgumentDelta(argument_delta) => {
        self
          .calls
          .entry(update.index)
          .or_default()
          .argument_delta(&argument_delta);

        Ok(Vec::new())
      }
      ToolCallUpdateKind::Finish => self
        .calls
        .remove(&update.index)
        .map(ToolCallBuilder::finish)
        .transpose()
        .map(Option::into_iter)
        .map(Iterator::collect),
      ToolCallUpdateKind::Id(id) => {
        self.calls.entry(update.index).or_default().id(id);

        Ok(Vec::new())
      }
      ToolCallUpdateKind::Name(name) => {
        self.calls.entry(update.index).or_default().name(name);

        Ok(Vec::new())
      }
    }
  }

  pub(crate) fn push_event<E>(&mut self, event: E) -> Result<Vec<RawToolCall>>
  where
    E: ToolCallStreamEvent<Index = I>,
  {
    let mut tool_calls = Vec::new();

    for update in event.tool_call_updates() {
      tool_calls.extend(self.push(update)?);
    }

    Ok(tool_calls)
  }
}

impl<I: Ord> Default for ToolCallStream<I> {
  fn default() -> Self {
    Self {
      calls: BTreeMap::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Event {
    updates: Vec<ToolCallUpdate<usize>>,
  }

  impl ToolCallStreamEvent for Event {
    type Index = usize;

    fn tool_call_updates(self) -> Vec<ToolCallUpdate<Self::Index>> {
      self.updates
    }
  }

  #[test]
  fn anthropic_tool_call_arguments_are_streamed_as_deltas() {
    let mut stream = ToolCallStream::<usize>::default();

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockStart {
          content_block: anthropic::ContentBlock::ToolUse {
            id: "foo".into(),
            input: json!({}),
            name: "read_file".into(),
          },
          index: 0,
        })
        .unwrap(),
      Vec::new(),
    );

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockDelta {
          delta: anthropic::ContentBlockDelta::InputJsonDelta {
            partial_json: r#"{"path":"#.into(),
          },
          index: 0,
        })
        .unwrap(),
      Vec::new(),
    );

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockDelta {
          delta: anthropic::ContentBlockDelta::InputJsonDelta {
            partial_json: r#""foo"}"#.into(),
          },
          index: 0,
        })
        .unwrap(),
      Vec::new(),
    );

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockStop {
          index: 0,
        })
        .unwrap(),
      vec![RawToolCall::new("foo", "read_file", json!({"path": "foo"}))],
    );
  }

  #[test]
  fn event_can_finish_multiple_calls() {
    let mut stream = ToolCallStream::default();

    stream
      .push(ToolCallUpdate {
        index: 0,
        kind: ToolCallUpdateKind::Id("foo".into()),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate {
        index: 0,
        kind: ToolCallUpdateKind::Name("read_file".into()),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate {
        index: 0,
        kind: ToolCallUpdateKind::ArgumentDelta(r#"{"path":"foo"}"#.into()),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate {
        index: 1,
        kind: ToolCallUpdateKind::Id("bar".into()),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate {
        index: 1,
        kind: ToolCallUpdateKind::Name("read_file".into()),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate {
        index: 1,
        kind: ToolCallUpdateKind::ArgumentDelta(r#"{"path":"bar"}"#.into()),
      })
      .unwrap();

    assert_eq!(
      stream
        .push_event(Event {
          updates: vec![
            ToolCallUpdate {
              index: 0,
              kind: ToolCallUpdateKind::Finish,
            },
            ToolCallUpdate {
              index: 1,
              kind: ToolCallUpdateKind::Finish,
            },
          ],
        })
        .unwrap(),
      vec![
        RawToolCall::new("foo", "read_file", json!({"path": "foo"})),
        RawToolCall::new("bar", "read_file", json!({"path": "bar"})),
      ],
    );
  }
}
