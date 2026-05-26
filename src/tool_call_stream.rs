use super::*;

pub(crate) struct ToolCallStream<I> {
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

  pub(crate) fn push(
    &mut self,
    update: ToolCallUpdate<I>,
  ) -> Result<Vec<RawToolCall>> {
    match update.kind {
      ToolCallUpdateKind::ArgumentDelta(argument_delta) => {
        let tool_call = self
          .calls
          .remove(&update.index)
          .unwrap_or_default()
          .argument_delta(&argument_delta);

        self.calls.insert(update.index, tool_call);

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
        let tool_call =
          self.calls.remove(&update.index).unwrap_or_default().id(id);

        self.calls.insert(update.index, tool_call);

        Ok(Vec::new())
      }
      ToolCallUpdateKind::Name(name) => {
        let tool_call = self
          .calls
          .remove(&update.index)
          .unwrap_or_default()
          .name(name);

        self.calls.insert(update.index, tool_call);

        Ok(Vec::new())
      }
    }
  }

  pub(crate) fn push_event<E>(&mut self, event: E) -> Result<Vec<RawToolCall>>
  where
    E: ToolCallStreamEvent<Index = I>,
  {
    event.tool_call_updates().into_iter().try_fold(
      Vec::new(),
      |mut tool_calls, update| {
        tool_calls.extend(self.push(update)?);
        Ok(tool_calls)
      },
    )
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
