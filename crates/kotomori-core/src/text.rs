use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Text {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub additional_params: Option<Value>,
  pub text: String,
}
