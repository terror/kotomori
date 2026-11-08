use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Text {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) additional_params: Option<Value>,
  pub(crate) text: String,
}
