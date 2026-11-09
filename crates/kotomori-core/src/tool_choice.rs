use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
  Auto,
  None,
  Required,
  Specific(Vec<ToolName>),
}
