use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ToolRequest {
  #[serde(default)]
  pub additional_params: Option<Value>,
  pub function: ToolFunction,
  pub id: CallId,
  #[serde(default)]
  pub signature: Option<String>,
}

impl ToolRequest {
  #[must_use]
  pub fn new(id: CallId, function: ToolFunction) -> Self {
    Self {
      additional_params: None,
      function,
      id,
      signature: None,
    }
  }
}
