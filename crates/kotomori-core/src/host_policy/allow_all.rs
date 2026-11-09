use super::*;

#[derive(Clone, Debug, Default)]
pub struct AllowAll;

impl HostPolicy for AllowAll {
  fn evaluate<'a>(
    &'a self,
    _context: &'a RunContext,
    _invocation: &'a ToolRequest,
    _approval: ApprovalPolicy,
  ) -> BoxFuture<'a, Result<PolicyDecision>> {
    Box::pin(async { Ok(PolicyDecision::Allow) })
  }
}
