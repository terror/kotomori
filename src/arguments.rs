use super::*;

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Arguments {
  #[command(flatten)]
  options: Options,
  #[command(subcommand)]
  subcommand: Option<Subcommand>,
}

impl Arguments {
  pub(crate) async fn run(self) -> Result {
    let options = self.options;

    match self.subcommand {
      Some(subcommand) => subcommand.run(options).await,
      None => App::new(&options)?.run().await,
    }
  }
}
