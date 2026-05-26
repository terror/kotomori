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
      ToolCallFragment::ArgumentFragment {
        argument_fragment,
        index,
      } => {
        let tool_call = self
          .calls
          .remove(&index)
          .unwrap_or_default()
          .argument_fragment(&argument_fragment);

        self.calls.insert(index, tool_call);

        Ok(None)
      }
      ToolCallFragment::Arguments { arguments, index } => {
        let tool_call = self
          .calls
          .remove(&index)
          .unwrap_or_default()
          .arguments(arguments);

        self.calls.insert(index, tool_call);

        Ok(None)
      }
      ToolCallFragment::Finish { index } => self
        .calls
        .remove(&index)
        .map(ToolCallBuilder::finish)
        .transpose(),
      ToolCallFragment::Id { id, index } => {
        let tool_call = self.calls.remove(&index).unwrap_or_default().id(id);

        self.calls.insert(index, tool_call);

        Ok(None)
      }
      ToolCallFragment::Name { index, name } => {
        let tool_call =
          self.calls.remove(&index).unwrap_or_default().name(name);

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
      .tool_call_fragments()
      .into_iter()
      .try_fold(None, |_, fragment| self.push(fragment))
  }
}

impl<I: Ord> Default for ToolCallStream<I> {
  fn default() -> Self {
    Self {
      calls: BTreeMap::new(),
    }
  }
}
