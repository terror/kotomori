use super::*;

#[derive(Clone, Debug, Default)]
pub struct DefaultHostPolicy;

impl HostPolicy for DefaultHostPolicy {
  fn evaluate(
    &self,
    _invocation: &ToolRequest,
    approval: ApprovalPolicy,
  ) -> PolicyDecision {
    match approval {
      ApprovalPolicy::None => PolicyDecision::Allow,
      ApprovalPolicy::Required => PolicyDecision::Ask,
    }
  }
}
