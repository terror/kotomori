use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ToolDefinition {
  pub(crate) description: String,
  pub(crate) name: String,
  pub(crate) parameters: Value,
}
