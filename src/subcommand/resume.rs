use super::*;

#[derive(Args, Debug)]
pub(crate) struct Resume {
  #[arg(long, help = "Resume the most recent session")]
  last: bool,
}

impl Resume {
  pub(crate) async fn run(
    self,
    database: Database,
    settings: Settings,
  ) -> Result {
    let sessions = database.get_sessions(&settings.directory)?;

    let Some(last_id) = sessions.first().map(|session| session.id) else {
      println!("No saved sessions.");
      return Ok(());
    };

    let mut app = App::new(database, &settings)?;

    if self.last {
      app.resume(last_id)?;
    } else {
      app.set_screen(Screen::Resume(ResumePicker::new(sessions)))?;
    }

    app.run().await
  }
}
