use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallId {
  Local(String),
  Provider {
    call_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    item_id: Option<String>,
  },
}

impl CallId {
  #[must_use]
  pub fn correlation(&self) -> &str {
    match self {
      Self::Local(id) | Self::Provider { call_id: id, .. } => id,
    }
  }

  pub fn new(call_id: impl Into<String>) -> Self {
    Self::Provider {
      call_id: call_id.into(),
      item_id: None,
    }
  }

  pub(super) fn validate(&self) -> Result {
    match self {
      Self::Local(id) => ensure!(
        Uuid::try_parse(id).is_ok_and(|uuid| uuid.to_string() == *id),
        "local tool call identity must be a canonical UUID"
      ),
      Self::Provider { call_id, .. } => ensure!(
        !call_id.is_empty(),
        "provider tool call identity must not be empty"
      ),
    }

    Ok(())
  }
}
