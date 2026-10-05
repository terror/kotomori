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
  const DATABASE_NAME: &'static str = "kotomori.db";

  pub(crate) async fn run(self) -> Result {
    let settings = Settings::resolve(self.options, &Config::load()?)?;

    let root = if let Some(path) = env::var_os("KOTOMORI_HOME") {
      PathBuf::from(path)
    } else if let Some(path) = env::var_os("XDG_STATE_HOME") {
      PathBuf::from(path).join("kotomori")
    } else {
      env::home_dir()
        .context("failed to determine home directory")?
        .join(".local/state/kotomori")
    };

    let database = Database::new(&root.join(Self::DATABASE_NAME))?;

    match self.subcommand {
      Some(subcommand) => subcommand.run(database, settings).await,
      None => App::new(database, &settings)?.run().await,
    }
  }
}
