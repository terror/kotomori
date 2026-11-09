use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ProviderResponse {
  pub finish_reason: Option<FinishReason>,
  pub message: AgentMessage,
  pub usage: Usage,
}

impl ProviderResponse {
  pub(super) fn validate(&self) -> Result {
    if self
      .finish_reason
      .as_ref()
      .and_then(FinishReason::outcome)
      .is_some()
    {
      return Ok(());
    }

    let mut calls = BTreeSet::new();

    for content in &self.message.content {
      if let AssistantContent::ToolCall(call) = content {
        call.id.validate()?;

        ensure!(
          calls.insert(call.id.correlation()),
          "duplicate tool call in provider response"
        );
      }
    }

    Ok(())
  }
}
