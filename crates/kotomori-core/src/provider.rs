use super::*;

pub trait Provider: fmt::Debug + Send + Sync {
  fn stream<'a>(
    &'a self,
    request: Request<'a>,
    sink: &'a mut ProviderSink,
  ) -> BoxFuture<'a, Result<AgentMessage>>;
}
