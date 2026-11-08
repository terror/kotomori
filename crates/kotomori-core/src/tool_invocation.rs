use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ToolInvocation {
  pub protocol: ToolRequest,
}
