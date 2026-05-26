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
    match update {
      ToolCallUpdate::ArgumentDelta {
        argument_delta,
        index,
      } => {
        let tool_call = self
          .calls
          .remove(&index)
          .unwrap_or_default()
          .argument_delta(&argument_delta);

        self.calls.insert(index, tool_call);

        Ok(Vec::new())
      }
      ToolCallUpdate::Arguments { arguments, index } => {
        let tool_call = self
          .calls
          .remove(&index)
          .unwrap_or_default()
          .arguments(arguments);

        self.calls.insert(index, tool_call);

        Ok(Vec::new())
      }
      ToolCallUpdate::Finish { index } => self
        .calls
        .remove(&index)
        .map(ToolCallBuilder::finish)
        .transpose()
        .map(Option::into_iter)
        .map(Iterator::collect),
      ToolCallUpdate::Id { id, index } => {
        let tool_call = self.calls.remove(&index).unwrap_or_default().id(id);

        self.calls.insert(index, tool_call);

        Ok(Vec::new())
      }
      ToolCallUpdate::Name { index, name } => {
        let tool_call =
          self.calls.remove(&index).unwrap_or_default().name(name);

        self.calls.insert(index, tool_call);

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
      .push(ToolCallUpdate::Id {
        id: "foo".into(),
        index: 0,
      })
      .unwrap();

    stream
      .push(ToolCallUpdate::Name {
        index: 0,
        name: "read_file".into(),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate::Arguments {
        arguments: json!({"path": "foo"}),
        index: 0,
      })
      .unwrap();

    stream
      .push(ToolCallUpdate::Id {
        id: "bar".into(),
        index: 1,
      })
      .unwrap();

    stream
      .push(ToolCallUpdate::Name {
        index: 1,
        name: "read_file".into(),
      })
      .unwrap();

    stream
      .push(ToolCallUpdate::Arguments {
        arguments: json!({"path": "bar"}),
        index: 1,
      })
      .unwrap();

    assert_eq!(
      stream
        .push_event(Event {
          updates: vec![
            ToolCallUpdate::Finish { index: 0 },
            ToolCallUpdate::Finish { index: 1 },
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
