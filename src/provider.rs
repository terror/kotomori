use super::*;

mod fake;
mod rig;

pub(crate) use {fake::Fake, rig::Rig};

#[async_trait]
pub(crate) trait Provider: fmt::Debug + Send + Sync {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result;
}
