use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ProviderResponse {
  pub finish_reason: Option<FinishReason>,
  pub message: AgentMessage,
  pub usage: Usage,
}

impl ProviderResponse {
  pub(super) fn validate(&self) -> Result {
    let mut calls = Vec::<&CallId>::new();

    for content in &self.message.content {
      if let AssistantContent::ToolCall(call) = content {
        ensure!(
          !calls.contains(&&call.id),
          "duplicate tool call in provider response"
        );

        calls.push(&call.id);
      }
    }

    Ok(())
  }
}
