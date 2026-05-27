use super::*;

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Arguments {
  #[command(flatten)]
  options: Options,
}

impl Arguments {
  pub(crate) async fn run(self) -> Result {
    App::new(&self.options)?.run().await
  }
}
