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
  ) -> Result<Option<RawToolCall>> {
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

        Ok(None)
      }
      ToolCallUpdate::Arguments { arguments, index } => {
        let tool_call = self
          .calls
          .remove(&index)
          .unwrap_or_default()
          .arguments(arguments);

        self.calls.insert(index, tool_call);

        Ok(None)
      }
      ToolCallUpdate::Finish { index } => self
        .calls
        .remove(&index)
        .map(ToolCallBuilder::finish)
        .transpose(),
      ToolCallUpdate::Id { id, index } => {
        let tool_call = self.calls.remove(&index).unwrap_or_default().id(id);

        self.calls.insert(index, tool_call);

        Ok(None)
      }
      ToolCallUpdate::Name { index, name } => {
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
      .tool_call_updates()
      .into_iter()
      .try_fold(None, |_, update| self.push(update))
  }
}

impl<I: Ord> Default for ToolCallStream<I> {
  fn default() -> Self {
    Self {
      calls: BTreeMap::new(),
    }
  }
}
