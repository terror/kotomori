use {super::*, resume::Resume};

mod resume;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Resume a previous session")]
  Resume(Resume),
}

impl Subcommand {
  pub(crate) async fn run(self, settings: Settings) -> Result {
    match self {
      Self::Resume(resume) => resume.run(settings).await,
    }
  }
}
