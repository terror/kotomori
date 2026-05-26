use super::*;

pub(crate) enum ToolCallFragment<I> {
  ArgumentFragment { argument_fragment: String, index: I },
  Arguments { arguments: Value, index: I },
  Finish { index: I },
  Id { id: String, index: I },
  Name { index: I, name: String },
}
