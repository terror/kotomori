use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolDefinition {
  pub description: String,
  pub name: ToolName,
  pub parameters: Value,
}
