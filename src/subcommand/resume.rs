use super::*;

#[derive(Args, Debug)]
pub(crate) struct Resume {
  #[arg(long, help = "Resume the most recent session")]
  last: bool,
}

impl Resume {
  pub(crate) async fn run(self, settings: Settings) -> Result {
    let sessions = Database::new()?.get_sessions()?;

    let Some(last_id) = sessions.first().map(|session| session.id) else {
      println!("No saved sessions.");
      return Ok(());
    };

    let mut app =
      App::with_screen(&settings, Screen::Resume(ResumePicker::new(sessions)))?;

    if self.last {
      app.resume(last_id)?;
    }

    app.run().await
  }
}
