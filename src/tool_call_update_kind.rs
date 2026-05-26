use super::*;

pub(crate) enum ToolCallUpdateKind {
  ArgumentDelta(String),
  Arguments(Value),
  Finish,
  Id(String),
  Name(String),
}
