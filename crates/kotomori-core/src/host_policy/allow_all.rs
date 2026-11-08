use super::*;

#[derive(Clone, Debug, Default)]
pub struct AllowAll;

impl HostPolicy for AllowAll {
  fn evaluate(
    &self,
    _invocation: &ToolRequest,
    _approval: ApprovalPolicy,
  ) -> PolicyDecision {
    PolicyDecision::Allow
  }
}
