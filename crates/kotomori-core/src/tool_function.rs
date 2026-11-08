use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ToolFunction {
  pub arguments: Value,
  pub name: ToolName,
}
