use super::*;

pub trait Provider: fmt::Debug + Send + Sync {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a ProviderSink,
  ) -> BoxFuture<'a, Result<ProviderResponse>>;
}
