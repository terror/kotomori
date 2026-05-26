use super::*;

pub(crate) struct ToolCallUpdate<I> {
  pub(crate) index: I,
  pub(crate) kind: ToolCallUpdateKind,
}
