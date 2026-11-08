use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct ToolRequest {
  #[serde(default)]
  pub(crate) additional_params: Option<Value>,
  pub(crate) function: ToolFunction,
  pub(crate) id: CallId,
  #[serde(default)]
  pub(crate) signature: Option<String>,
}

impl ToolRequest {
  pub(crate) fn new(id: CallId, function: ToolFunction) -> Self {
    Self {
      additional_params: None,
      function,
      id,
      signature: None,
    }
  }

  #[cfg(test)]
  pub(crate) fn with_additional_params(
    mut self,
    additional_params: Option<Value>,
  ) -> Self {
    self.additional_params = additional_params;
    self
  }

  #[cfg(test)]
  pub(crate) fn with_signature(mut self, signature: Option<String>) -> Self {
    self.signature = signature;
    self
  }
}
