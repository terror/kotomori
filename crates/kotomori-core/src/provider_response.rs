use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ProviderResponse {
  pub finish_reason: Option<FinishReason>,
  pub message: AgentMessage,
  pub usage: Usage,
}
