use {super::*, resume::Resume};

mod resume;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Resume a previous session", alias = "r")]
  Resume(Resume),
}

impl Subcommand {
  pub(crate) async fn run(
    self,
    database: Database,
    settings: Settings,
  ) -> Result {
    match self {
      Self::Resume(resume) => resume.run(database, settings).await,
    }
  }
}
