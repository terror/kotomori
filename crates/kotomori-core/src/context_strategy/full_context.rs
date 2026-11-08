use super::*;

#[derive(Clone, Debug, Default)]
pub struct FullContext;

impl ContextStrategy for FullContext {
  fn prepare<'a>(
    &'a self,
    request: Request<'a>,
  ) -> BoxFuture<'a, Result<RequestContext<'a>>> {
    Box::pin(async move { Ok(request.into()) })
  }
}
