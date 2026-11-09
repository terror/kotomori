use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CustomEntry {
  pub data: Value,
  pub kind: String,
  pub namespace: String,
  pub version: u32,
}
