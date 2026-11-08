use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CallId {
  Local(String),
  Provider {
    call_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    item_id: Option<String>,
  },
}

impl CallId {
  pub(crate) fn new(call_id: impl Into<String>) -> Self {
    Self::Provider {
      call_id: call_id.into(),
      item_id: None,
    }
  }
}
