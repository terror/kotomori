use super::*;

pub(crate) async fn run(mut settings: Settings) -> Result {
  let sessions = SessionStore::list()?;

  if sessions.is_empty() {
    println!("No saved sessions.");
    return Ok(());
  }

  let Some(path) = ResumePicker::new(sessions).run()? else {
    return Ok(());
  };

  let session = SessionStore::load(&path)?;

  settings.model = session.file.model.parse().with_context(|| {
    format!("failed to parse session model {}", session.file.model)
  })?;

  App::with_state(&settings, State::with_session(&settings, session)?)?
    .run()
    .await
}
