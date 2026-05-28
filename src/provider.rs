use super::*;

mod mock;
mod rig;

pub(crate) use {mock::Mock, rig::Rig};

#[async_trait]
pub(crate) trait Provider: fmt::Debug + Send + Sync {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result;
}
