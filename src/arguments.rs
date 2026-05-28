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
    let settings = Settings::resolve(self.options, &Config::load()?)?;

    match self.subcommand {
      Some(subcommand) => subcommand.run(settings).await,
      None => App::new(&settings)?.run().await,
    }
  }
}
