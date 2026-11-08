use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct ToolFunction {
  pub(crate) arguments: Value,
  pub(crate) name: ToolName,
}
