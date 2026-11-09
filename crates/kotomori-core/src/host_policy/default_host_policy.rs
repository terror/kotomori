use super::*;

#[derive(Clone, Debug, Default)]
pub struct DefaultHostPolicy;

impl HostPolicy for DefaultHostPolicy {
  fn evaluate<'a>(
    &'a self,
    _context: &'a RunContext,
    _invocation: &'a ToolRequest,
    approval: ApprovalPolicy,
  ) -> BoxFuture<'a, Result<PolicyDecision>> {
    Box::pin(async move {
      Ok(match approval {
        ApprovalPolicy::None => PolicyDecision::Allow,
        ApprovalPolicy::Required => PolicyDecision::Ask,
      })
    })
  }
}
