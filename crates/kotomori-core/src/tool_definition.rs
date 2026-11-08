use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolDefinition {
  pub description: String,
  pub name: String,
  pub parameters: Value,
}
