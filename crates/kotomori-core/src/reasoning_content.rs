use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", content = "content", rename_all = "snake_case")]
pub enum ReasoningContent {
  Encrypted(String),
  Redacted {
    data: String,
  },
  Summary(String),
  Text {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
  },
}
