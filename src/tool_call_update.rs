use super::*;

pub(crate) enum ToolCallUpdate<I> {
  ArgumentDelta { argument_delta: String, index: I },
  Arguments { arguments: Value, index: I },
  Finish { index: I },
  Id { id: String, index: I },
  Name { index: I, name: String },
}
