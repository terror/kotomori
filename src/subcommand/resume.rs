use super::*;

pub(crate) async fn run(settings: Settings) -> Result {
  let sessions = SessionStore::list()?;

  if sessions.is_empty() {
    println!("No saved sessions.");
    return Ok(());
  }

  App::with_screen(&settings, Screen::Resume(ResumePicker::new(sessions)))?
    .run()
    .await
}
