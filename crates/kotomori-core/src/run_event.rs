use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RunEvent {
  pub event: AgentEvent,
  pub run_id: RunId,
}
