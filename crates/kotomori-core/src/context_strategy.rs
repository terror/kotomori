use super::*;

mod full_context;

pub use full_context::FullContext;

pub trait ContextStrategy: Debug + Send + Sync {
  fn prepare<'a>(
    &'a self,
    context: &'a RunContext,
    request: Request<'a>,
  ) -> BoxFuture<'a, Result<RequestContext<'a>>>;
}
