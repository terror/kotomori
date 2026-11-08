use super::*;

pub trait ContextStrategy: Debug + Send + Sync {
  fn prepare<'a>(
    &'a self,
    request: Request<'a>,
  ) -> BoxFuture<'a, Result<RequestContext<'a>>>;
}
