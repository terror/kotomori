use super::*;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolResult {
  pub content: String,
  pub details: Value,
  pub outcome: ToolOutcome,
}

impl ToolResult {
  #[must_use]
  pub fn message_content(&self) -> String {
    serde_json::json!({"content": self.content, "outcome": self.outcome})
      .to_string()
  }

  #[must_use]
  pub fn output(&self) -> Option<&str> {
    (!self.content.is_empty()).then_some(&self.content)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn model_content_excludes_details() {
    let result = ToolResult {
      content: "foo".into(),
      details: serde_json::json!({"bar": "baz"}),
      outcome: ToolOutcome::Success,
    };

    assert_eq!(
      result.message_content(),
      r#"{"content":"foo","outcome":"success"}"#,
    );
  }
}
