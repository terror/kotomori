pub(crate) enum ToolCallUpdateKind {
  ArgumentDelta(String),
  Finish,
  Id(String),
  Name(String),
}
