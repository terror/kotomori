use super::*;

#[derive(Args, Debug)]
pub(crate) struct Resume {
  #[arg(long, help = "Resume the most recent session")]
  last: bool,
}

impl Resume {
  pub(crate) async fn run(self, settings: Settings) -> Result {
    let sessions = Database::new()?.get_sessions()?;

    if sessions.is_empty() {
      println!("No saved sessions.");
      return Ok(());
    }

    let last_id = sessions.first().and_then(|session| session.id);

    let mut app =
      App::with_screen(&settings, Screen::Resume(ResumePicker::new(sessions)))?;

    if self.last {
      app.resume(last_id.context("saved session is missing an ID")?)?;
    }

    app.run().await
  }
}
