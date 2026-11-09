use super::*;

mod allow_all;
mod default_host_policy;
mod policy_chain;

pub use {
  allow_all::AllowAll, default_host_policy::DefaultHostPolicy,
  policy_chain::PolicyChain,
};

pub trait HostPolicy: Debug + Send + Sync {
  fn evaluate<'a>(
    &'a self,
    context: &'a RunContext,
    invocation: &'a ToolRequest,
    approval: ApprovalPolicy,
  ) -> BoxFuture<'a, Result<PolicyDecision>>;
}
