use super::*;

pub(crate) enum ToolCallFragment<I> {
  Finish {
    index: I,
  },
  Update {
    argument_fragment: Option<String>,
    arguments: Option<Value>,
    id: Option<String>,
    index: I,
    name: Option<String>,
  },
}
