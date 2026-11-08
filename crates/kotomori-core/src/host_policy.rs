use super::*;

mod allow_all;
mod default_host_policy;

pub use {allow_all::AllowAll, default_host_policy::DefaultHostPolicy};

pub trait HostPolicy: Debug + Send + Sync {
  fn evaluate(
    &self,
    invocation: &ToolRequest,
    approval: ApprovalPolicy,
  ) -> PolicyDecision;
}
