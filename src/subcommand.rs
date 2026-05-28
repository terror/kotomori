use super::*;

mod resume;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Resume a previous session")]
  Resume,
}

impl Subcommand {
  pub(crate) async fn run(self, options: Options) -> Result {
    match self {
      Self::Resume => resume::run(options).await,
    }
  }
}
