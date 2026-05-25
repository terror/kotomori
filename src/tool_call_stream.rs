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
    fragment: ToolCallFragment<I>,
  ) -> Result<Option<RawToolCall>> {
    match fragment {
      ToolCallFragment::Finish { index } => self
        .calls
        .remove(&index)
        .map(ToolCallBuilder::finish)
        .transpose(),
      ToolCallFragment::Update {
        argument_fragment,
        arguments,
        id,
        index,
        name,
      } => {
        let tool_call = self
          .calls
          .remove(&index)
          .unwrap_or_default()
          .id(id)
          .name(name)
          .arguments(arguments)
          .argument_fragment(argument_fragment.as_deref());

        self.calls.insert(index, tool_call);

        Ok(None)
      }
    }
  }

  pub(crate) fn push_event<E>(
    &mut self,
    event: E,
  ) -> Result<Option<RawToolCall>>
  where
    E: ToolCallStreamEvent<Index = I>,
  {
    event
      .tool_call_fragment()
      .map_or(Ok(None), |fragment| self.push(fragment))
  }
}

impl<I: Ord> Default for ToolCallStream<I> {
  fn default() -> Self {
    Self {
      calls: BTreeMap::new(),
    }
  }
}
